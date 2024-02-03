use core::marker::PhantomData;

use super::{
    primitive::{FieldCopyAccess, FieldView},
    Field, StorageIntoFieldView, StorageToFieldView,
};

/// Implementing the [LayoutAs] trait for a custom type allows that custom type to be used
/// as the type of a layout field. Note that the value of this type is copied each time it
/// is accessed, so this is only recommended for primitive wrappers of primitive types,
/// not for types that are expensive to copy.
///
/// # Example
/// ```
/// use binary_layout::{prelude::*, LayoutAs};
///
/// struct MyIdType(u64);
/// impl LayoutAs<u64> for MyIdType {
///   fn read(v: u64) -> MyIdType {
///     MyIdType(v)
///   }
///
///   fn write(v: MyIdType) -> u64 {
///     v.0
///   }
/// }
///
/// define_layout!(my_layout, BigEndian, {
///   // ... other fields ...
///   field: MyIdType as u64,
///   // ... other fields ...
/// });
///
/// # fn main() {}
/// ```
pub trait LayoutAs<U> {
    /// Implement this to define how the custom type is constructed from the underlying type
    /// after it was read from a layouted binary slice.
    fn read(v: U) -> Self;

    /// Implement this to define how the custom type is converted into the underlying type
    /// so it can be written into a layouted binary slice.
    fn write(v: Self) -> U;
}

/// A [WrappedField] is a [Field] that, unlike [PrimitiveField](crate::PrimitiveField), does not directly represent a primitive type.
/// Instead, it represents a wrapper type that can be converted to/from a primitive type using the [LayoutAs] trait.
/// See [Field] for more info on this API.
///
/// # Example:
/// ```
/// use binary_layout::{prelude::*, LayoutAs};
///
/// struct MyIdType(u64);
/// impl LayoutAs<u64> for MyIdType {
///   fn read(v: u64) -> MyIdType {
///     MyIdType(v)
///   }
///
///   fn write(v: MyIdType) -> u64 {
///     v.0
///   }
/// }
///
/// define_layout!(my_layout, BigEndian, {
///   // ... other fields ...
///   field: MyIdType as u64,
///   // ... other fields ...
/// });
///
/// fn func(storage_data: &mut [u8]) {
///   // read some data
///   let read_data: MyIdType = my_layout::field::read(storage_data);
///   // equivalent: let read_data = MyIdType(u16::from_le_bytes((&storage_data[0..2]).try_into().unwrap()));
///
///   // write some data
///   my_layout::field::write(storage_data, MyIdType(10));
///   // equivalent: data_slice[18..22].copy_from_slice(&10u32.to_le_bytes());
/// }
///
/// # fn main() {
/// #   let mut storage = [0; 1024];
/// #   func(&mut storage);
/// # }
/// ```
pub struct WrappedField<U, T: LayoutAs<U>, F: Field> {
    _p1: PhantomData<U>,
    _p2: PhantomData<T>,
    _p3: PhantomData<F>,
}

impl<U, T: LayoutAs<U>, F: Field> Field for WrappedField<U, T, F> {
    /// See [Field::Endian]
    type Endian = F::Endian;
    /// See [Field::OFFSET]
    const OFFSET: usize = F::OFFSET;
    /// See [Field::SIZE]
    const SIZE: Option<usize> = F::SIZE;
}

impl<
        'a,
        U,
        T: LayoutAs<U>,
        F: FieldCopyAccess<HighLevelType = U> + StorageToFieldView<&'a [u8]>,
    > StorageToFieldView<&'a [u8]> for WrappedField<U, T, F>
{
    type View = FieldView<&'a [u8], Self>;

    #[inline(always)]
    fn view(storage: &'a [u8]) -> Self::View {
        Self::View::new(storage)
    }
}

impl<
        'a,
        U,
        T: LayoutAs<U>,
        F: FieldCopyAccess<HighLevelType = U> + StorageToFieldView<&'a mut [u8]>,
    > StorageToFieldView<&'a mut [u8]> for WrappedField<U, T, F>
{
    type View = FieldView<&'a mut [u8], Self>;

    #[inline(always)]
    fn view(storage: &'a mut [u8]) -> Self::View {
        Self::View::new(storage)
    }
}

impl<
        U,
        S: AsRef<[u8]>,
        T: LayoutAs<U>,
        F: FieldCopyAccess<HighLevelType = U> + StorageIntoFieldView<S>,
    > StorageIntoFieldView<S> for WrappedField<U, T, F>
{
    type View = FieldView<S, Self>;

    #[inline(always)]
    fn into_view(storage: S) -> Self::View {
        Self::View::new(storage)
    }
}

impl<U, T: LayoutAs<U>, F: FieldCopyAccess<HighLevelType = U>> FieldCopyAccess
    for WrappedField<U, T, F>
{
    /// See [FieldCopyAccess::HighLevelType]
    type HighLevelType = T;

    /// Read the field from a given data region, assuming the defined layout, using the [Field] API.
    ///
    /// # Example:
    /// ```
    /// use binary_layout::{prelude::*, LayoutAs};
    ///
    /// #[derive(Debug, PartialEq, Eq)]
    /// struct MyIdType(u64);
    /// impl LayoutAs<u64> for MyIdType {
    ///   fn read(v: u64) -> MyIdType {
    ///     MyIdType(v)
    ///   }
    ///
    ///   fn write(v: MyIdType) -> u64 {
    ///     v.0
    ///   }
    /// }
    ///
    /// define_layout!(my_layout, LittleEndian, {
    ///   //... other fields ...
    ///   some_integer_field: MyIdType as u64,
    ///   //... other fields ...
    /// });
    ///
    /// fn func(storage_data: &mut [u8]) {
    ///   my_layout::some_integer_field::write(storage_data, MyIdType(50));
    ///   assert_eq!(MyIdType(50), my_layout::some_integer_field::read(storage_data));
    /// }
    ///
    /// # fn main() {
    /// #   let mut storage = [0; 1024];
    /// #   func(&mut storage);
    /// # }
    /// ```
    #[inline(always)]
    fn read(storage: &[u8]) -> Self::HighLevelType {
        let v = F::read(storage);
        <T as LayoutAs<U>>::read(v)
    }

    /// Write the field to a given data region, assuming the defined layout, using the [Field] API.
    ///
    /// # Example:
    /// See [FieldCopyAccess::read] for an example
    #[inline(always)]
    fn write(storage: &mut [u8], v: Self::HighLevelType) {
        let v = <T as LayoutAs<U>>::write(v);
        F::write(storage, v)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::float_cmp)]
    use crate::prelude::*;
    use crate::{LayoutAs, PrimitiveField, WrappedField};
    use core::convert::TryInto;

    #[derive(Debug, PartialEq, Eq)]
    struct Wrapped<T>(T);
    impl<T> LayoutAs<T> for Wrapped<T> {
        fn read(v: T) -> Self {
            Self(v)
        }
        fn write(v: Self) -> T {
            v.0
        }
    }

    macro_rules! test_wrapped_field {
        ($type:ty, $expected_size:expr, $value1:expr, $value2:expr) => {
            test_wrapped_field!(@case, $type, $expected_size, $value1, $value2, little, LittleEndian, from_le_bytes);
            test_wrapped_field!(@case, $type, $expected_size, $value1, $value2, big, BigEndian, from_be_bytes);
            test_wrapped_field!(@case, $type, $expected_size, $value1, $value2, native, NativeEndian, from_ne_bytes);
        };
        (@case, $type:ty, $expected_size:expr, $value1:expr, $value2: expr, $endian:ident, $endian_type:ty, $endian_fn:ident) => {
            $crate::internal::paste! {
                #[test]
                fn [<test_ $type _ $endian endian>]() {
                    let mut storage = vec![0; 1024];

                    type Field1 = WrappedField<$type, Wrapped<$type>, PrimitiveField<$type, $endian_type, 5>>;
                    type Field2 = WrappedField<$type, Wrapped<$type>, PrimitiveField<$type, $endian_type, 123>>;

                    Field1::write(&mut storage, Wrapped($value1));
                    Field2::write(&mut storage, Wrapped($value2));

                    assert_eq!(Wrapped($value1), Field1::read(&storage));
                    assert_eq!(Wrapped($value2), Field2::read(&storage));

                    assert_eq!($value1, <$type>::$endian_fn((&storage[5..(5+$expected_size)]).try_into().unwrap()));
                    assert_eq!(
                        $value2,
                        <$type>::$endian_fn((&storage[123..(123+$expected_size)]).try_into().unwrap())
                    );

                    assert_eq!(Some($expected_size), Field1::SIZE);
                    assert_eq!(5, Field1::OFFSET);
                    assert_eq!(Some($expected_size), Field2::SIZE);
                    assert_eq!(123, Field2::OFFSET);
                }
            }
        };
    }

    test_wrapped_field!(i8, 1, 50, -20);
    test_wrapped_field!(i16, 2, 500, -2000);
    test_wrapped_field!(i32, 4, 10i32.pow(8), -(10i32.pow(7)));
    test_wrapped_field!(i64, 8, 10i64.pow(15), -(10i64.pow(14)));
    test_wrapped_field!(i128, 16, 10i128.pow(30), -(10i128.pow(28)));

    test_wrapped_field!(u8, 1, 50, 20);
    test_wrapped_field!(u16, 2, 500, 2000);
    test_wrapped_field!(u32, 4, 10u32.pow(8), (10u32.pow(7)));
    test_wrapped_field!(u64, 8, 10u64.pow(15), (10u64.pow(14)));
    test_wrapped_field!(u128, 16, 10u128.pow(30), (10u128.pow(28)));

    test_wrapped_field!(f32, 4, 10f32.powf(8.31), -(10f32.powf(7.31)));
    test_wrapped_field!(f64, 8, 10f64.powf(15.31), -(10f64.powf(15.31)));

    macro_rules! test_wrapped_unit_field {
        ($endian:ident, $endian_type:ty) => {
            $crate::internal::paste! {
                #[allow(clippy::unit_cmp)]
                #[test]
                fn [<test_unit_ $endian endian>]() {
                    let mut storage = vec![0; 1024];

                    type Field1 = WrappedField<(), Wrapped<()>, PrimitiveField<(), LittleEndian, 5>>;
                    type Field2 = WrappedField<(), Wrapped<()>, PrimitiveField<(), LittleEndian, 123>>;

                    Field1::write(&mut storage, Wrapped(()));
                    Field2::write(&mut storage, Wrapped(()));

                    assert_eq!(Wrapped(()), Field1::read(&storage));
                    assert_eq!(Wrapped(()), Field2::read(&storage));

                    assert_eq!(Some(0), Field1::SIZE);
                    assert_eq!(5, Field1::OFFSET);
                    assert_eq!(Some(0), Field2::SIZE);
                    assert_eq!(123, Field2::OFFSET);

                    // Zero-sized types do not mutate the storage, so it should remain
                    // unchanged for all of time.
                    assert_eq!(storage, vec![0; 1024]);
                }
            }
        };
    }

    test_wrapped_unit_field!(little, LittleEndian);
    test_wrapped_unit_field!(big, BigEndian);
    test_wrapped_unit_field!(native, NativeEndian);
}
