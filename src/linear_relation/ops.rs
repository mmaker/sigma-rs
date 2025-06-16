use core::ops::{Add, Mul, Neg, Sub};

use super::{GroupVar, LinearCombination, ScalarVar, Term};

impl Add<LinearCombination> for LinearCombination {
    type Output = Self;

    fn add(mut self, mut rhs: LinearCombination) -> Self {
        self.0.append(&mut rhs.0);
        self
    }
}

impl Add<Term> for LinearCombination {
    type Output = LinearCombination;

    fn add(mut self, rhs: Term) -> LinearCombination {
        self.0.push(rhs);
        self
    }
}

impl Add<LinearCombination> for Term {
    type Output = LinearCombination;

    fn add(self, rhs: LinearCombination) -> LinearCombination {
        rhs + self
    }
}

impl Add<Term> for Term {
    type Output = LinearCombination;

    fn add(self, rhs: Term) -> LinearCombination {
        LinearCombination::from(self) + rhs
    }
}

impl Mul<ScalarVar> for GroupVar {
    type Output = Term;

    fn mul(self, rhs: ScalarVar) -> Term {
        Term {
            elem: self,
            scalar: rhs,
        }
    }
}

impl Mul<GroupVar> for ScalarVar {
    type Output = Term;

    fn mul(self, rhs: GroupVar) -> Term {
        Term {
            scalar: self,
            elem: rhs,
        }
    }
}

// GroupVar algebraic operations
impl Add<GroupVar> for GroupVar {
    type Output = LinearCombination;

    fn add(self, rhs: GroupVar) -> LinearCombination {
        LinearCombination::from(Term {
            elem: self,
            scalar: ScalarVar(0), // coefficient 1 (assumes scalar variable 0 represents 1)
        }) + Term {
            elem: rhs,
            scalar: ScalarVar(0),
        }
    }
}

impl Sub<GroupVar> for GroupVar {
    type Output = LinearCombination;

    fn sub(self, rhs: GroupVar) -> LinearCombination {
        LinearCombination::from(Term {
            elem: self,
            scalar: ScalarVar(0), // coefficient 1
        }) + Term {
            elem: rhs,
            scalar: ScalarVar(1), // coefficient -1
        }
    }
}

// ScalarVar algebraic operations (assuming GroupVar(0) is the generator)
impl Add<ScalarVar> for ScalarVar {
    type Output = LinearCombination;

    fn add(self, rhs: ScalarVar) -> LinearCombination {
        // Represents (self + rhs) * G where G is the generator at GroupVar(0)
        LinearCombination::from(Term {
            elem: GroupVar(0), // generator
            scalar: self,
        }) + Term {
            elem: GroupVar(0), // generator
            scalar: rhs,
        }
    }
}

impl Sub<ScalarVar> for ScalarVar {
    type Output = LinearCombination;

    fn sub(self, rhs: ScalarVar) -> LinearCombination {
        LinearCombination::from(Term {
            elem: GroupVar(0), // generator
            scalar: self,
        }) + Term {
            elem: GroupVar(0), // generator
            scalar: rhs,
        }
    }
}

impl Neg for ScalarVar {
    type Output = Term;

    fn neg(self) -> Term {
        // Represents -scalar * G where G is the generator at GroupVar(0)
        Term {
            elem: GroupVar(0), // generator
            scalar: self,
        }
    }
}
