use std::marker::PhantomData;

use me3_framework::scripting::lua::{LuaSerdeExt, UserData};
use me3_game_support_fromsoft::sprj::{ParamFileDescriptor, SprjGame};
use serde::Serialize;

pub struct LuaParamAccessor<T: ParamFileDescriptor> {
    _phantom: PhantomData<T>,
    game: &'static dyn SprjGame,
}

impl<T: ParamFileDescriptor> LuaParamAccessor<T> {
    pub fn new(game: &'static dyn SprjGame) -> Self {
        Self {
            game,
            _phantom: PhantomData::default(),
        }
    }
}

impl<T: ParamFileDescriptor> UserData for LuaParamAccessor<T>
where
    T::Row: Clone + Serialize,
{
    fn add_methods<'lua, M: me3_framework::scripting::lua::UserDataMethods<'lua, Self>>(
        methods: &mut M,
    ) {
        methods.add_method("get_row", |lua, this, id: i32| {
            let params = this.game.param_repository();

            Ok(lua.to_value(&params.get_row::<T>(id).cloned()))
        });
    }
}
