use std::{path::Path, sync::Mutex};

use mlua::{Lua, MultiValue, Table, UserData};
use once_cell::sync::OnceCell;

use crate::{FrameworkError, FrameworkGlobal};

pub mod lua {
    pub use mlua::*;
}

pub struct ScriptHost {
    lua: Mutex<Lua>,
}

impl FrameworkGlobal for ScriptHost {
    fn cell() -> &'static OnceCell<Self> {
        static INSTANCE: OnceCell<ScriptHost> = OnceCell::new();
        &INSTANCE
    }

    fn create() -> Result<Self, FrameworkError> {
        Ok(ScriptHost {
            lua: Mutex::new(Lua::new()),
        })
    }
}

pub trait ScriptType: UserData + Send + Sync {}
impl<T: UserData + Send + Sync> ScriptType for T {}

impl ScriptHost {
    pub fn eval<
        S: AsRef<str>,
        F: FnOnce(Result<MultiValue, mlua::Error>) -> Result<T, FrameworkError>,
        T,
    >(
        &'_ self,
        code: S,
        result_handler: F,
    ) -> Result<T, FrameworkError> {
        result_handler(
            self.lua
                .lock()
                .unwrap()
                .load(code.as_ref())
                .eval::<MultiValue>(),
        )
    }

    pub fn load_script<P>(&self, path: P) -> Result<(), FrameworkError>
    where
        P: AsRef<Path>,
    {
        let script_text = std::fs::read_to_string(path.as_ref())?;
        let _: mlua::Value = self
            .lua
            .lock()
            .expect("lua lock was poisoned")
            .load(&script_text)
            .eval()
            .unwrap(); // TODO: propagate error

        Ok(())
    }

    pub fn set_table<S, F>(&self, name: S, constructor: F) -> Result<(), FrameworkError>
    where
        S: AsRef<str>,
        F: FnOnce(Table) -> Result<Table, mlua::Error>,
    {
        let lua = self.lua.lock().expect("lua lock was poisoned");
        let globals = lua.globals();

        let _ = globals.set(
            name.as_ref(),
            constructor(
                lua.create_table()
                    .expect("failed basic lua operation: create_table"),
            )?,
        );

        Ok(())
    }

    pub fn set<S, T>(&self, name: S, value: T)
    where
        S: AsRef<str>,
        T: ScriptType + 'static,
    {
        let lua = self.lua.lock().expect("lua lock was poisoned");
        let globals = lua.globals();

        // TODO: propagate this error?
        let _ = globals.set(name.as_ref(), value);
    }
}
