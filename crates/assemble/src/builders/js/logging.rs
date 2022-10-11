/// Create logging binding
use rquickjs::bind;

#[bind(object, public)]
#[quickjs(bare)]
mod logger {
    pub struct Logger {}

    impl Logger {
        pub fn new() -> Self {
            Self {}
        }

        pub fn info(&self, string: String) {
            info!("{}", string)
        }
    }
}
