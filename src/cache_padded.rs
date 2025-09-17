/// Vendored and simplified from crossbeam-utils.
use core::fmt;
use core::ops::{Deref, DerefMut};

// Starting from Intel's Sandy Bridge, spatial prefetcher is now pulling pairs of 64-byte cache
// lines at a time, so we have to align to 128 bytes rather than 64.
//
// Sources:
// - https://www.intel.com/content/dam/www/public/us/en/documents/manuals/64-ia-32-architectures-optimization-manual.pdf
// - https://github.com/facebook/folly/blob/1b5288e6eea6df074758f877c849b6e73bbb9fbb/folly/lang/Align.h#L107
//
// ARM's big.LITTLE architecture has asymmetric cores and "big" cores have 128 byte cache line size
// Sources:
// - https://www.mono-project.com/news/2016/09/12/arm64-icache/
//
#[cfg_attr(
    any(target_arch = "x86_64", target_arch = "aarch64"),
    repr(align(128))
)]
#[cfg_attr(
    not(any(target_arch = "x86_64", target_arch = "aarch64")),
    repr(align(64))
)]
#[derive(Default, PartialEq, Eq)]
pub struct CachePadded<T> {
    value: T,
}

#[allow(unsafe_code)]
unsafe impl<T: Send> Send for CachePadded<T> {}

#[allow(unsafe_code)]
unsafe impl<T: Sync> Sync for CachePadded<T> {}

impl<T> CachePadded<T> {
    /// Pads and aligns a value to the length of a cache line.
    pub const fn new(t: T) -> CachePadded<T> {
        CachePadded::<T> { value: t }
    }
}

impl<T> Deref for CachePadded<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.value
    }
}

impl<T> DerefMut for CachePadded<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.value
    }
}

impl<T: fmt::Debug> fmt::Debug for CachePadded<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CachePadded").field("value", &self.value).finish()
    }
}

impl<T> From<T> for CachePadded<T> {
    fn from(t: T) -> Self {
        CachePadded::new(t)
    }
}

/*
// c++ implementation
#include <cstddef>  // for std::size_t
#include <type_traits>  // for std::aligned_storage
#include <utility>  // for std::move

// Cache line size
#if defined(__x86_64__) || defined(_M_X64) || defined(__aarch64__)
constexpr std::size_t CACHE_LINE_SIZE = 128;
#else
constexpr std::size_t CACHE_LINE_SIZE = 64;
#endif

// CachePadded class
template <typename T>
class CachePadded {
public:
    // Constructors
    CachePadded() {
        new (&storage) T(); // Default construct T
    }

    explicit CachePadded(const T& value) {
        new (&storage) T(value); // Copy construct T
    }

    explicit CachePadded(T&& value) {
        new (&storage) T(std::move(value)); // Move construct T
    }

    // Destructor
    ~CachePadded() {
        get().~T(); // Manually destroy the object
    }

    // Access the wrapped object
    T& get() {
        return *reinterpret_cast<T*>(&storage);
    }

    const T& get() const {
        return *reinterpret_cast<const T*>(&storage);
    }

    // Delete copy/move constructors and assignment operators for safety
    CachePadded(const CachePadded&) = delete;
    CachePadded& operator=(const CachePadded&) = delete;
    CachePadded(CachePadded&&) = delete;
    CachePadded& operator=(CachePadded&&) = delete;

private:
    // Aligned storage for the object
    alignas(CACHE_LINE_SIZE) std::aligned_storage_t<sizeof(T), alignof(T)> storage;
};

 */