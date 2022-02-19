use super::*;

macro_rules! impl_index_traits {
    ($id_type:ty, $output_type:ty, $arena:ident) => {
        impl<A, B, C> std::ops::Index<$id_type> for Graph<A, B, C> {
            type Output = $output_type;

            fn index(&self, index: $id_type) -> &Self::Output {
                self.$arena.get(index).unwrap_or_else(|| {
                    panic!(
                        "{} index error for {:?}. Has the value been deleted?",
                        stringify!($id_type),
                        index
                    )
                })
            }
        }

        impl<A, B, C> std::ops::IndexMut<$id_type> for Graph<A, B, C> {
            fn index_mut(&mut self, index: $id_type) -> &mut Self::Output {
                self.$arena.get_mut(index).unwrap_or_else(|| {
                    panic!(
                        "{} index error for {:?}. Has the value been deleted?",
                        stringify!($id_type),
                        index
                    )
                })
            }
        }
    };
}

impl_index_traits!(NodeId, Node<A>, nodes);
impl_index_traits!(InputId, InputParam<B, C>, inputs);
impl_index_traits!(OutputId, OutputParam<B>, outputs);
