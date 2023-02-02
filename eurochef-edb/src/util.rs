#[macro_export]
macro_rules! structure_size_tests {
    ($($typename:path = $size:expr),*) => {
        #[cfg(test)]
        mod tests {
            use super::*;

            #[test]
            fn assert_struct_sizes() {
                $(
                   assert_eq!(std::mem::size_of::<$typename>(), $size);
                )*
            }
        }
    };
}
