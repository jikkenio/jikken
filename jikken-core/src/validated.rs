use nonempty_collections::NEVec;
use validated::{
    Validated,
    Validated::{Fail, Good},
};

pub trait ValidatedExt<T, E> {
    fn map5<U, V, W, X, Z, F>(
        self,
        vu: Validated<U, E>,
        vv: Validated<V, E>,
        vw: Validated<W, E>,
        vx: Validated<X, E>,
        f: F,
    ) -> Validated<Z, E>
    where
        F: FnOnce(T, U, V, W, X) -> Z;
}

impl<T, E> ValidatedExt<T, E> for Validated<T, E> {
    /// Maps a function over five `Validated`, but only if all five are of the
    /// `Good` variant. If any failed, then their errors are concatenated.
    fn map5<U, V, W, X, Z, F>(
        self,
        vu: Validated<U, E>,
        vv: Validated<V, E>,
        vw: Validated<W, E>,
        vx: Validated<X, E>,
        f: F,
    ) -> Validated<Z, E>
    where
        F: FnOnce(T, U, V, W, X) -> Z,
    {
        match (self, vu, vv, vw, vx) {
            (Good(t), Good(u), Good(v), Good(w), Good(x)) => Good(f(t, u, v, w, x)),

            (Good(_), Good(_), Good(_), Good(_), Fail(e)) => Fail(e),
            (Good(_), Good(_), Good(_), Fail(e), Good(_)) => Fail(e),
            (Good(_), Good(_), Fail(e), Good(_), Good(_)) => Fail(e),
            (Good(_), Fail(e), Good(_), Good(_), Good(_)) => Fail(e),
            (Fail(e), Good(_), Good(_), Good(_), Good(_)) => Fail(e),

            (Good(_), Good(_), Good(_), Fail(e0), Fail(e1)) => Fail(nons(e0, Some(e1).into_iter())),
            (Good(_), Good(_), Fail(e0), Good(_), Fail(e1)) => Fail(nons(e0, Some(e1).into_iter())),
            (Good(_), Fail(e0), Good(_), Good(_), Fail(e1)) => Fail(nons(e0, Some(e1).into_iter())),
            (Fail(e0), Good(_), Good(_), Good(_), Fail(e1)) => Fail(nons(e0, Some(e1).into_iter())),
            (Good(_), Good(_), Fail(e0), Fail(e1), Good(_)) => Fail(nons(e0, Some(e1).into_iter())),
            (Good(_), Fail(e0), Good(_), Fail(e1), Good(_)) => Fail(nons(e0, Some(e1).into_iter())),
            (Fail(e0), Good(_), Good(_), Fail(e1), Good(_)) => Fail(nons(e0, Some(e1).into_iter())),
            (Good(_), Fail(e0), Fail(e1), Good(_), Good(_)) => Fail(nons(e0, Some(e1).into_iter())),
            (Fail(e0), Good(_), Fail(e1), Good(_), Good(_)) => Fail(nons(e0, Some(e1).into_iter())),
            (Fail(e0), Fail(e1), Good(_), Good(_), Good(_)) => Fail(nons(e0, Some(e1).into_iter())),

            (Good(_), Good(_), Fail(e0), Fail(e1), Fail(e2)) => {
                Fail(nons(e0, vec![e1, e2].into_iter()))
            }
            (Good(_), Fail(e0), Good(_), Fail(e1), Fail(e2)) => {
                Fail(nons(e0, vec![e1, e2].into_iter()))
            }
            (Fail(e0), Good(_), Good(_), Fail(e1), Fail(e2)) => {
                Fail(nons(e0, vec![e1, e2].into_iter()))
            }
            (Good(_), Fail(e0), Fail(e1), Good(_), Fail(e2)) => {
                Fail(nons(e0, vec![e1, e2].into_iter()))
            }
            (Fail(e0), Good(_), Fail(e1), Good(_), Fail(e2)) => {
                Fail(nons(e0, vec![e1, e2].into_iter()))
            }
            (Fail(e0), Fail(e1), Good(_), Good(_), Fail(e2)) => {
                Fail(nons(e0, vec![e1, e2].into_iter()))
            }
            (Good(_), Fail(e0), Fail(e1), Fail(e2), Good(_)) => {
                Fail(nons(e0, vec![e1, e2].into_iter()))
            }
            (Fail(e0), Good(_), Fail(e1), Fail(e2), Good(_)) => {
                Fail(nons(e0, vec![e1, e2].into_iter()))
            }
            (Fail(e0), Fail(e1), Good(_), Fail(e2), Good(_)) => {
                Fail(nons(e0, vec![e1, e2].into_iter()))
            }
            (Fail(e0), Fail(e1), Fail(e2), Good(_), Good(_)) => {
                Fail(nons(e0, vec![e1, e2].into_iter()))
            }

            (Good(_), Fail(e0), Fail(e1), Fail(e2), Fail(e3)) => {
                Fail(nons(e0, vec![e1, e2, e3].into_iter()))
            }
            (Fail(e0), Good(_), Fail(e1), Fail(e2), Fail(e3)) => {
                Fail(nons(e0, vec![e1, e2, e3].into_iter()))
            }
            (Fail(e0), Fail(e1), Good(_), Fail(e2), Fail(e3)) => {
                Fail(nons(e0, vec![e1, e2, e3].into_iter()))
            }
            (Fail(e0), Fail(e1), Fail(e2), Good(_), Fail(e3)) => {
                Fail(nons(e0, vec![e1, e2, e3].into_iter()))
            }
            (Fail(e0), Fail(e1), Fail(e2), Fail(e3), Good(_)) => {
                Fail(nons(e0, vec![e1, e2, e3].into_iter()))
            }

            (Fail(e0), Fail(e1), Fail(e2), Fail(e3), Fail(e4)) => {
                Fail(nons(e0, vec![e1, e2, e3, e4].into_iter()))
            }
        }
    }
}

/// Fuse some `NEVec`s together.
fn nons<E, I>(mut a: NEVec<E>, rest: I) -> NEVec<E>
where
    I: Iterator<Item = NEVec<E>>,
{
    for mut i in rest {
        a.push(i.head);
        a.append(&mut i.tail)
    }

    a
}
