use std::any::Any;

pub trait AsAny {
    fn as_any(&self) -> &(dyn Any + '_);
}

pub trait AsAnyMut: 'static + AsAny {
    fn as_any_mut(&mut self) -> &mut dyn Any;
}
