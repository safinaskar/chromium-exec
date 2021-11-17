#![allow(clippy::needless_return)]
#![allow(clippy::missing_safety_doc)]
#![allow(clippy::redundant_field_names)]
#![allow(clippy::result_unit_err)]
#![deny(unsafe_op_in_unsafe_fn)]
#![deny(clippy::as_conversions)] // Потому что "as" может сконвертить что угодно куда угодно

#![deny(unsafe_code)]

// Лимит, указанный в документации: 1 MB, т. е. 1024 * 1024
// Нужно:
// array_size * 4 + 100 <= 1024 * 1024
// array_size * 4 <= 1024 * 1024 - 100
// array_size <= (1024 * 1024 - 100) / 4
pub const CHUNK_SIZE: usize = (1024 * 1024 - 100) / 4;
