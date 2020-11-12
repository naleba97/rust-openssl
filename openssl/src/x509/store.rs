//! Describe a context in which to verify an `X509` certificate.
//!
//! The `X509` certificate store holds trusted CA certificates used to verify
//! peer certificates.
//!
//! # Example
//!
//! ```rust
//! use openssl::x509::store::{X509StoreBuilder, X509Store};
//! use openssl::x509::{X509, X509Name};
//! use openssl::pkey::PKey;
//! use openssl::hash::MessageDigest;
//! use openssl::rsa::Rsa;
//! use openssl::nid::Nid;
//!
//! let rsa = Rsa::generate(2048).unwrap();
//! let pkey = PKey::from_rsa(rsa).unwrap();
//!
//! let mut name = X509Name::builder().unwrap();
//! name.append_entry_by_nid(Nid::COMMONNAME, "foobar.com").unwrap();
//! let name = name.build();
//!
//! let mut builder = X509::builder().unwrap();
//! builder.set_version(2).unwrap();
//! builder.set_subject_name(&name).unwrap();
//! builder.set_issuer_name(&name).unwrap();
//! builder.set_pubkey(&pkey).unwrap();
//! builder.sign(&pkey, MessageDigest::sha256()).unwrap();
//!
//! let certificate: X509 = builder.build();
//!
//! let mut builder = X509StoreBuilder::new().unwrap();
//! let _ = builder.add_cert(certificate);
//!
//! let store: X509Store = builder.build();
//! ```

use cfg_if::cfg_if;
use foreign_types::ForeignTypeRef;
use libc::c_int;
use std::ffi::CString;
use std::mem;
use std::path::Path;

use crate::error::ErrorStack;
use crate::stack::StackRef;
#[cfg(any(ossl102, libressl261))]
use crate::x509::verify::X509VerifyFlags;
use crate::x509::{X509Object, X509};
use crate::{cvt, cvt_p};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct X509Purpose(c_int);

impl X509Purpose {
    pub const SSL_CLIENT: X509Purpose = X509Purpose(ffi::X509_PURPOSE_SSL_CLIENT);
    pub const SSL_SERVER: X509Purpose = X509Purpose(ffi::X509_PURPOSE_SSL_SERVER);
    pub const NS_SSL_SERVER: X509Purpose = X509Purpose(ffi::X509_PURPOSE_NS_SSL_SERVER);
    pub const SMIME_SIGN: X509Purpose = X509Purpose(ffi::X509_PURPOSE_SMIME_SIGN);
    pub const SMIME_ENCRYPT: X509Purpose = X509Purpose(ffi::X509_PURPOSE_SMIME_ENCRYPT);
    pub const CRL_SIGN: X509Purpose = X509Purpose(ffi::X509_PURPOSE_CRL_SIGN);
    pub const ANY: X509Purpose = X509Purpose(ffi::X509_PURPOSE_ANY);
    pub const OCSP_HELPER: X509Purpose = X509Purpose(ffi::X509_PURPOSE_OCSP_HELPER);
    pub const TIMESTAMP_SIGN: X509Purpose = X509Purpose(ffi::X509_PURPOSE_TIMESTAMP_SIGN);

    /// Constructs a `X509Purpose` from a raw OpenSSL value.
    pub fn from_raw(raw: c_int) -> X509Purpose {
        X509Purpose(raw)
    }

    /// Returns the raw OpenSSL value represented by this type.
    #[allow(clippy::trivially_copy_pass_by_ref)]
    pub fn as_raw(&self) -> c_int {
        self.0
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct X509Trust(c_int);

impl X509Trust {
    pub const DEFAULT: X509Trust = X509Trust(ffi::X509_TRUST_DEFAULT);
    pub const COMPAT: X509Trust = X509Trust(ffi::X509_TRUST_COMPAT);
    pub const SSL_CLIENT: X509Trust = X509Trust(ffi::X509_TRUST_SSL_CLIENT);
    pub const SSL_SERVER: X509Trust = X509Trust(ffi::X509_TRUST_SSL_SERVER);
    pub const EMAIL: X509Trust = X509Trust(ffi::X509_TRUST_EMAIL);
    pub const OBJECT_SIGN: X509Trust = X509Trust(ffi::X509_TRUST_OBJECT_SIGN);
    pub const OCSP_SIGN: X509Trust = X509Trust(ffi::X509_TRUST_OCSP_SIGN);
    pub const OCSP_REQUEST: X509Trust = X509Trust(ffi::X509_TRUST_OCSP_REQUEST);
    pub const TSA: X509Trust = X509Trust(ffi::X509_TRUST_TSA);

    /// Constructs a `X509Trust` from a raw OpenSSL value.
    pub fn from_raw(raw: c_int) -> X509Trust {
        X509Trust(raw)
    }

    /// Returns the raw OpenSSL value represented by this type.
    #[allow(clippy::trivially_copy_pass_by_ref)]
    pub fn as_raw(&self) -> c_int {
        self.0
    }
}

foreign_type_and_impl_send_sync! {
    type CType = ffi::X509_STORE;
    fn drop = ffi::X509_STORE_free;

    /// A builder type used to construct an `X509Store`.
    pub struct X509StoreBuilder;
    /// Reference to an `X509StoreBuilder`.
    pub struct X509StoreBuilderRef;
}

impl X509StoreBuilder {
    /// Returns a builder for a certificate store.
    ///
    /// The store is initially empty.
    pub fn new() -> Result<X509StoreBuilder, ErrorStack> {
        unsafe {
            ffi::init();

            cvt_p(ffi::X509_STORE_new()).map(X509StoreBuilder)
        }
    }

    /// Constructs the `X509Store`.
    pub fn build(self) -> X509Store {
        let store = X509Store(self.0);
        mem::forget(self);
        store
    }
}

impl X509StoreBuilderRef {
    /// Adds a certificate to the certificate store.
    // FIXME should take an &X509Ref
    pub fn add_cert(&mut self, cert: X509) -> Result<(), ErrorStack> {
        unsafe { cvt(ffi::X509_STORE_add_cert(self.as_ptr(), cert.as_ptr())).map(|_| ()) }
    }

    /// Sets the maximum verification depth, or the maximum number of intermediate CA certificates that can appear in a chain.
    ///
    /// This corresponds to [`X509_STORE_set_depth`].
    ///
    /// [`X509_STORE_set_depth`]: https://www.openssl.org/docs/man1.1.1/man3/X509_STORE_set_depth.html
    pub fn set_depth(&mut self, depth: i32) -> Result<(), ErrorStack> {
        unsafe { cvt(ffi::X509_STORE_set_depth(self.as_ptr(), depth)).map(|_| ()) }
    }

    /// Sets the purpose used to verify the certificate chain.
    ///
    /// This corresponds to [`X509_STORE_set_purpose`].
    ///
    /// [`X509_STORE_set_purpose`]: https://www.openssl.org/docs/man1.1.1/man3/X509_STORE_set_purpose.html
    pub fn set_purpose(&mut self, purpose: X509Purpose) -> Result<(), ErrorStack> {
        unsafe { cvt(ffi::X509_STORE_set_purpose(self.as_ptr(), purpose.as_raw())).map(|_| ()) }
    }

    /// Sets the trust value used to verify the certificate chain.
    ///
    /// This corresponds to [`X509_STORE_set_trust`].
    ///
    /// [`X509_STORE_set_trust`]: https://www.openssl.org/docs/man1.1.1/man3/X509_STORE_set_trust.html
    pub fn set_trust(&mut self, trust: X509Trust) -> Result<(), ErrorStack> {
        unsafe { cvt(ffi::X509_STORE_set_trust(self.as_ptr(), trust.as_raw())).map(|_| ()) }
    }

    /// Load trusted certificate(s) into the `X509Store` from a file or a directory.
    ///
    /// The certificates in the file or directory should be in a hashed format.
    pub fn load_locations<P: AsRef<Path>>(&mut self, file: P, dir: P) -> Result<(), ErrorStack> {
        let file = CString::new(file.as_ref().as_os_str().to_str().unwrap()).unwrap();
        let dir = CString::new(dir.as_ref().as_os_str().to_str().unwrap()).unwrap();

        unsafe {
            cvt(ffi::X509_STORE_load_locations(
                self.as_ptr(),
                file.as_ptr() as *const _,
                dir.as_ptr() as *const _,
            ))
            .map(|_| ())
        }
    }

    /// Load certificates from their default locations.
    ///
    /// These locations are read from the `SSL_CERT_FILE` and `SSL_CERT_DIR`
    /// environment variables if present, or defaults specified at OpenSSL
    /// build time otherwise.
    pub fn set_default_paths(&mut self) -> Result<(), ErrorStack> {
        unsafe { cvt(ffi::X509_STORE_set_default_paths(self.as_ptr())).map(|_| ()) }
    }

    /// Adds a lookup method to the store.
    ///
    /// This corresponds to [`X509_STORE_add_lookup`].
    ///
    /// [`X509_STORE_add_lookup`]: https://www.openssl.org/docs/man1.1.1/man3/X509_STORE_add_lookup.html
    pub fn add_lookup<T>(
        &mut self,
        method: &'static X509LookupMethodRef<T>,
    ) -> Result<&mut X509LookupRef<T>, ErrorStack> {
        let lookup = unsafe { ffi::X509_STORE_add_lookup(self.as_ptr(), method.as_ptr()) };
        cvt_p(lookup).map(|ptr| unsafe { X509LookupRef::from_ptr_mut(ptr) })
    }

    /// Sets certificate chain validation related flags.
    ///
    /// This corresponds to [`X509_STORE_set_flags`].
    ///
    /// [`X509_STORE_set_flags`]: https://www.openssl.org/docs/man1.1.1/man3/X509_STORE_set_flags.html
    #[cfg(any(ossl102, libressl261))]
    pub fn set_flags(&mut self, flags: X509VerifyFlags) -> Result<(), ErrorStack> {
        unsafe { cvt(ffi::X509_STORE_set_flags(self.as_ptr(), flags.bits())).map(|_| ()) }
    }
}

generic_foreign_type_and_impl_send_sync! {
    type CType = ffi::X509_LOOKUP;
    fn drop = ffi::X509_LOOKUP_free;

    /// Information used by an `X509Store` to look up certificates and CRLs.
    pub struct X509Lookup<T>;
    /// Reference to an `X509Lookup`.
    pub struct X509LookupRef<T>;
}

/// Marker type corresponding to the [`X509_LOOKUP_hash_dir`] lookup method.
///
/// [`X509_LOOKUP_hash_dir`]: https://www.openssl.org/docs/man1.1.0/crypto/X509_LOOKUP_hash_dir.html
pub struct HashDir;

impl X509Lookup<HashDir> {
    /// Lookup method that loads certificates and CRLs on demand and caches
    /// them in memory once they are loaded. It also checks for newer CRLs upon
    /// each lookup, so that newer CRLs are used as soon as they appear in the
    /// directory.
    ///
    /// This corresponds to [`X509_LOOKUP_hash_dir`].
    ///
    /// [`X509_LOOKUP_hash_dir`]: https://www.openssl.org/docs/man1.1.0/crypto/X509_LOOKUP_hash_dir.html
    pub fn hash_dir() -> &'static X509LookupMethodRef<HashDir> {
        unsafe { X509LookupMethodRef::from_ptr(ffi::X509_LOOKUP_hash_dir()) }
    }
}

impl X509LookupRef<HashDir> {
    /// Specifies a directory from which certificates and CRLs will be loaded
    /// on-demand. Must be used with `X509Lookup::hash_dir`.
    ///
    /// This corresponds to [`X509_LOOKUP_add_dir`].
    ///
    /// [`X509_LOOKUP_add_dir`]: https://www.openssl.org/docs/man1.1.1/man3/X509_LOOKUP_add_dir.html
    pub fn add_dir(
        &mut self,
        name: &str,
        file_type: crate::ssl::SslFiletype,
    ) -> Result<(), ErrorStack> {
        let name = std::ffi::CString::new(name).unwrap();
        unsafe {
            cvt(ffi::X509_LOOKUP_add_dir(
                self.as_ptr(),
                name.as_ptr(),
                file_type.as_raw(),
            ))
            .map(|_| ())
        }
    }
}

generic_foreign_type_and_impl_send_sync! {
    type CType = ffi::X509_LOOKUP_METHOD;
    fn drop = X509_LOOKUP_meth_free;

    /// Method used to look up certificates and CRLs.
    pub struct X509LookupMethod<T>;
    /// Reference to an `X509LookupMethod`.
    pub struct X509LookupMethodRef<T>;
}

foreign_type_and_impl_send_sync! {
    type CType = ffi::X509_STORE;
    fn drop = ffi::X509_STORE_free;

    /// A certificate store to hold trusted `X509` certificates.
    pub struct X509Store;
    /// Reference to an `X509Store`.
    pub struct X509StoreRef;
}

impl X509StoreRef {
    /// Get a reference to the cache of certificates in this store.
    pub fn objects(&self) -> &StackRef<X509Object> {
        unsafe { StackRef::from_ptr(X509_STORE_get0_objects(self.as_ptr())) }
    }
}

cfg_if! {
    if #[cfg(any(ossl110, libressl270))] {
        use ffi::X509_STORE_get0_objects;
    } else {
        #[allow(bad_style)]
        unsafe fn X509_STORE_get0_objects(x: *mut ffi::X509_STORE) -> *mut ffi::stack_st_X509_OBJECT {
            (*x).objs
        }
    }
}

cfg_if! {
    if #[cfg(ossl110)] {
        use ffi::X509_LOOKUP_meth_free;
    } else {
        #[allow(bad_style)]
        unsafe fn X509_LOOKUP_meth_free(_x: *mut ffi::X509_LOOKUP_METHOD) {}
    }
}
