use std::marker::Tuple;

use seq_macro::seq;

pub trait Prepend<T>: Tuple {
    type Output: Tuple;

    fn prepend(self, value: T) -> Self::Output;
}

seq!(M in 1..16 {
    #(
        seq!(N in 0..M {
            impl<V, #(T~N,)*> Prepend<V> for (#(T~N,)*) {
                type Output = (V, #(T~N,)*);

                fn prepend(self, value: V) -> Self::Output {
                    (value, #(self.N,)*)
                }
            }
        });
    )*
});

impl<T> Prepend<T> for () {
    type Output = (T,);

    fn prepend(self, value: T) -> Self::Output {
        (value,)
    }
}

pub struct Curried<Closure, V> {
    closure: Closure,
    provider: fn() -> V,
}

impl<C, V> Curried<C, V> {
    pub fn new(closure: C, provider: fn() -> V) -> Self {
        Self { closure, provider }
    }
}

impl<Closure, Args, Ret, V> FnOnce<Args> for Curried<Closure, V>
where
    Closure: FnOnce<<Args as Prepend<V>>::Output, Output = Ret>,
    Args: Tuple + Prepend<V>,
{
    type Output = Ret;

    extern "rust-call" fn call_once(self, args: Args) -> Self::Output {
        self.closure.call_once(args.prepend((self.provider)()))
    }
}

impl<Closure, Args, Ret, V> Fn<Args> for Curried<Closure, V>
where
    Closure: Fn<<Args as Prepend<V>>::Output, Output = Ret>,
    Args: Tuple + Prepend<V>,
{
    extern "rust-call" fn call(&self, args: Args) -> Self::Output {
        self.closure.call(args.prepend((self.provider)()))
    }
}

impl<Closure, Args, Ret, V> FnMut<Args> for Curried<Closure, V>
where
    Closure: FnMut<<Args as Prepend<V>>::Output, Output = Ret>,
    Args: Tuple + Prepend<V>,
{
    extern "rust-call" fn call_mut(&mut self, args: Args) -> Self::Output {
        self.closure.call_mut(args.prepend((self.provider)()))
    }
}
