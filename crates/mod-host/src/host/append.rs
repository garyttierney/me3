use std::{cell::OnceCell, marker::Tuple};

use closure_ffi::{
    traits::{FnPtr, FnThunk, ToBoxedDyn},
    BareFnAny,
};
use seq_macro::seq;

pub trait Append<T>: Tuple {
    type Output: Tuple;

    fn append(self, value: T) -> Self::Output;
}

seq!(M in 1..13 {
    #(
        seq!(N in 0..M {
            impl<V, #(T~N,)*> Append<V> for (#(T~N,)*) {
                type Output = (#(T~N,)* V);

                fn append(self, value: V) -> Self::Output {
                    (#(self.N,)* value)
                }
            }
        });
    )*
});

impl<T> Append<T> for () {
    type Output = (T,);

    fn append(self, value: T) -> Self::Output {
        (value,)
    }
}

pub struct WithAppended<Closure, Appended>
where
    Appended: Clone + 'static,
{
    closure: Closure,
    appended: &'static OnceCell<Appended>,
}

impl<Closure, Appended> WithAppended<Closure, Appended>
where
    Appended: Clone + 'static,
{
    pub fn new(closure: Closure, appended: &'static OnceCell<Appended>) -> Self {
        Self { closure, appended }
    }

    pub fn bare<'a, B: FnPtr, S: ?Sized>(self) -> BareFnAny<B, S>
    where
        Self: ToBoxedDyn<S> + 'a,
        (B::CC, Self): FnThunk<B>,
    {
        BareFnAny::new(self)
    }
}

impl<Closure, Args, Ret, Appended: 'static> FnOnce<Args> for WithAppended<Closure, Appended>
where
    Closure: FnOnce<<Args as Append<Appended>>::Output, Output = Ret>,
    Args: Tuple + Append<Appended>,
    Appended: Clone + 'static,
{
    type Output = Ret;

    extern "rust-call" fn call_once(self, args: Args) -> Self::Output {
        self.closure.call_once(
            args.append(
                self.appended
                    .get()
                    .expect("appended value not initialized")
                    .clone(),
            ),
        )
    }
}

impl<Closure, Args, Ret, Appended: 'static> FnMut<Args> for WithAppended<Closure, Appended>
where
    Closure: Fn<<Args as Append<Appended>>::Output, Output = Ret>,
    Args: Tuple + Append<Appended>,
    Appended: Clone + 'static,
{
    extern "rust-call" fn call_mut(&mut self, args: Args) -> Self::Output {
        self.closure.call_mut(
            args.append(
                self.appended
                    .get()
                    .expect("appended value not initialized")
                    .clone(),
            ),
        )
    }
}

impl<Closure, Args, Ret, Appended: 'static> Fn<Args> for WithAppended<Closure, Appended>
where
    Closure: Fn<<Args as Append<Appended>>::Output, Output = Ret>,
    Args: Tuple + Append<Appended>,
    Appended: Clone + 'static,
{
    extern "rust-call" fn call(&self, args: Args) -> Self::Output {
        self.closure.call(
            args.append(
                self.appended
                    .get()
                    .expect("appended value not initialized")
                    .clone(),
            ),
        )
    }
}
