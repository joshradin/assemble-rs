//! Provide a "unified" way of adding plugins to an assemble project

use crate::project::error::ProjectResult;

use crate::project::Project;
use crate::utilities::Action;
use crate::BuildResult;
use parking_lot::RwLock;
use std::any::type_name;
use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;
use std::sync::Arc;

pub mod extensions;

/// A plugin to apply to the project. All plugins must implement default.
pub trait Plugin<T: ?Sized>: Default {
    /// Apply the plugin
    fn apply_to(&self, target: &mut T) -> ProjectResult;

    /// The id of the plugin. A plugin of a certain ID can only added once
    fn plugin_id(&self) -> &str {
        type_name::<Self>()
    }
}

pub trait PluginAware: Sized {
    /// Apply a plugin to this.
    fn apply_plugin<P: Plugin<Self>>(&mut self) -> ProjectResult {
        let mut manager = self.plugin_manager();
        manager.apply::<P>(self)
    }

    fn plugin_manager(&self) -> PluginManager<Self>;
}

/// A struct representing an applied plugin
pub struct PluginApplied;

type PluginManagerAction<T> = Box<dyn for<'a> FnOnce(&'a mut T) -> ProjectResult + Send + Sync>;

/// Facilities applying plugins and determining which plugins have been applied to
/// a plugin aware object.
pub struct PluginManager<T: PluginAware>(Arc<PluginManagerInner<T>>);

impl<T: PluginAware> Clone for PluginManager<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T : PluginAware> Default for PluginManager<T> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<T: PluginAware> Debug for PluginManager<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PluginManager")
            .finish_non_exhaustive()

    }
}

impl<T: PluginAware> PluginManager<T> {
    /// Check if this manager has a known plugin
    pub fn has_plugin(&self, id: &str) -> bool {
        self.0.has_plugin(id)
    }

    pub fn has_plugin_ty<P: Plugin<T>>(&self) -> bool {
        self.0.has_plugin_ty::<P>()
    }

    /// Applies this plugin if it hasn't been applied before
    pub fn apply<P: Plugin<T>>(&mut self, target: &mut T) -> ProjectResult {
        self.0.apply::<P>(target)
    }

    /// Set an action to perform if a plugin has been applied
    pub fn with_plugin<F: 'static>(&mut self, id: &str, target: &mut T, action: F) -> ProjectResult
    where
        T: 'static,
        for<'a> F: FnOnce(&'a mut T) -> ProjectResult + Send + Sync ,
    {
        self.0.with_plugin(id, target, action)
    }
}

struct PluginManagerInner<T: PluginAware> {
    applied: RwLock<HashSet<String>>,
    lazy_with_plugins: RwLock<HashMap<String, VecDeque<PluginManagerAction<T>>>>,
}

impl<T : PluginAware> Default for PluginManagerInner<T> {
    fn default() -> Self {
        Self {
            applied: Default::default(),
            lazy_with_plugins: Default::default()
        }
    }
}

impl<T: PluginAware> PluginManagerInner<T> {
    /// Check if this manager has a known plugin
    pub fn has_plugin(&self, id: &str) -> bool {
        self.applied.read().contains(id)
    }

    pub fn has_plugin_ty<P: Plugin<T>>(&self) -> bool {
        let plugins = P::default();
        let id = plugins.plugin_id();
        self.has_plugin(id)
    }

    /// Applies this plugin if it hasn't been applied before
    pub fn apply<P: Plugin<T>>(&self, target: &mut T) -> ProjectResult {
        let type_name: &str = std::any::type_name::<P>();

        trace!("attempting to apply plugin of type {type_name}");

        let ret = if self.has_plugin_ty::<P>() {
            trace!("plugin of type {type_name} already applied");
            Ok(())
        } else {
            let mut plugin = P::default();
            let id = plugin.plugin_id().to_string();
            trace!("applying generated plugin of type {type_name} with id {id}");
            plugin.apply_to(target)?;
            trace!("added applied plugin id {id}");
            self.applied.write().insert(id);

            Ok(())
        };
        for applied in self.applied.read().clone() {
            let mut lazy = self.lazy_with_plugins.write();
            if let Some(actions) = lazy.get_mut(&*applied) {
                let actions: Vec<_> = actions.drain(..).collect();
                trace!("found {} delayed actions for plugin {} that will now be applied", actions.len(), applied);
                for action in actions {
                    action.execute(target)?;
                }
            }
        }
        ret
    }

    /// Set an action to perform if a plugin has been applied
    pub fn with_plugin<F: 'static>(&self, id: &str, target: &mut T, action: F) -> ProjectResult
    where
        T: 'static,
        for<'a> F: FnOnce(&'a mut T) -> ProjectResult + Send + Sync,
    {
        if self.has_plugin(id) {
            action.execute(target)
        } else {
            let id = id.to_string();
            self.lazy_with_plugins
                .write()
                .entry(id)
                .or_default()
                .push_back(Box::new(action) as Box<dyn for<'b> FnOnce(&'b mut T) -> ProjectResult + Send + Sync>);
            Ok(())
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PluginError {
    #[error("Couldn't create the plugin")]
    CouldNotCreatePlugin,
}
