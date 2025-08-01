#[derive(Debug, PartialEq)]
pub enum Op {
    Minus,
    Plus,
}

#[derive(Debug)]
pub enum Node {
    Int(usize),
    Unary { op: Op, value: Box<Node> },
}

#[derive(PartialEq)]
pub enum State {
    Start,
    Int,
    Op,
}

fn parse_expr(expr: &str) -> Node {
    Node::Int(expr.parse::<usize>().unwrap())
}

// #[cfg(test)]
// mod tests {
//     #[test]
//     fn parse_int_node() {
//         let expr = "420";
//         let node = super::parse_expr(expr);

//         match node {
//             super::Node::Int(val) => assert_eq!(val, 420),
//             other => panic!("Expected Int node, got {:?}", other),
//         }
//     }

//     #[test]
//     fn parse_unary_minus_op() {
//         let expr = "-3";
//         let node = super::parse_expr(expr);

//         match node {
//             super::Node::Unary { op, value } => {
//                 assert_eq!(op, super::Op::Minus);
//                 match *value {
//                     super::Node::Int(val) => assert_eq!(val, 3),
//                     other => panic!("Expected inner Int node, got {:?}", other),
//                 }
//             }
//             other => panic!("Expected unary minus node, got {:?}", other),
//         }
//     }
// }
