//! Boolean expression trees with predicates at the leaves.
//!
//! Constructed trees can be evaluated against a substitute value to true or
//! false.

use serde::{Deserialize, Serialize};

/// Trait for type that can evaluate to true or false using a substituted
/// parameter.
pub trait Predicate {
    type Substitute;
    fn eval(&self, sub: &Self::Substitute) -> bool;
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum BoolExpr<P: Predicate> {
    And(Box<BoolExpr<P>>, Box<BoolExpr<P>>),
    Or(Box<BoolExpr<P>>, Box<BoolExpr<P>>),
    Not(Box<BoolExpr<P>>),
    Predicate(P),
}

impl<P: Predicate> BoolExpr<P> {
    pub fn eval(&self, sub: &P::Substitute) -> bool {
        match self {
            Self::And(left, right) => left.eval(sub) && right.eval(sub),
            Self::Or(left, right) => left.eval(sub) || right.eval(sub),
            Self::Not(expr) => !expr.eval(sub),
            Self::Predicate(p) => p.eval(sub),
        }
    }
}

impl<P: Predicate> From<P> for BoolExpr<P> {
    fn from(item: P) -> Self {
        BoolExpr::Predicate(item)
    }
}

impl<P: Predicate> From<P> for Box<BoolExpr<P>> {
    fn from(item: P) -> Self {
        Box::new(BoolExpr::Predicate(item))
    }
}

pub struct EqPredicate<T>(T);

impl<T: PartialEq> Predicate for EqPredicate<T> {
    type Substitute = T;
    fn eval(&self, sub: &T) -> bool {
        &self.0 == sub
    }
}

pub enum EqNePred<T> {
    Eq(T),
    Ne(T),
}

impl<T: PartialEq> Predicate for EqNePred<T> {
    type Substitute = T;
    fn eval(&self, sub: &T) -> bool {
        match self {
            Self::Eq(inner) => inner == sub,
            Self::Ne(inner) => inner != sub,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eq_predicate() {
        let expr =
            BoolExpr::Or(EqPredicate("a").into(), EqPredicate("b").into());
        assert!(expr.eval(&"a"));
        assert!(expr.eval(&"b"));
        assert!(!expr.eval(&"c"));
    }

    #[test]
    fn test_eq_ne_predicate() {
        // false if eq to "a" or "b", but not anything else
        let expr = BoolExpr::Not(
            BoolExpr::And(
                BoolExpr::Or(
                    EqNePred::Eq("a").into(),
                    EqNePred::Eq("b").into(),
                )
                .into(),
                EqNePred::Ne("c").into(),
            )
            .into(),
        );
        assert!(!expr.eval(&"a"));
        assert!(!expr.eval(&"b"));
        assert!(expr.eval(&"c"));
        assert!(expr.eval(&"d"));
    }
}
