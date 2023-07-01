pub trait BitExtensions {
    fn is_set(&self, index: usize) -> bool;
}

macro_rules! impl_bitext {
    ($type:ty) => {
        impl BitExtensions for $type {
            #[inline(always)]
            fn is_set(&self, index: usize) -> bool {
                (*self & (1 << index as $type)) != 0
            }
        }
    };
    ($($type:ty),+) => {
        $(
            impl_bitext!($type);
        )+
    };
}

impl_bitext!(u8, u16, u32, u64, i8, i16, i32, i64);
