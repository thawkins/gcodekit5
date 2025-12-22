use crate::model::{DesignPath, DesignerShape, Shape};
use csgrs::traits::CSG;

pub enum BooleanOp {
    Union,
    Difference,
    Intersection,
}

pub fn perform_boolean(a: &Shape, b: &Shape, op: BooleanOp) -> Shape {
    let csg_a = a.as_csg();
    let csg_b = b.as_csg();

    let result_csg = match op {
        BooleanOp::Union => csg_a.union(&csg_b),
        BooleanOp::Difference => csg_a.difference(&csg_b),
        BooleanOp::Intersection => csg_a.intersection(&csg_b),
    };

    Shape::Path(DesignPath::from_csg(result_csg))
}
