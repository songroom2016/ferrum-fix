//! FIX-over-TLS (FIXS) utilities.

mod iana2openssl;

use crate::openssl::ssl::*;
use iana2openssl::IANA_TO_OPENSSL;

/// Which version of FIX-over-TLS (FIXS) to use.
#[derive(Debug, Copy, Clone)]
pub enum Version {
    V1Draft,
}

impl Version {
    /// Returns a [`Vec`] of the suggested ciphersuites for TLS,
    /// according to `self` version. The ciphersuites are specified in IANA format.
    ///
    /// ```
    /// use fefix::fixs::Version;
    ///
    /// let version = Version::V1Draft;
    /// let ciphersuites_iana = version.recommended_cs_iana(false);
    /// assert!(ciphersuites_iana.iter().any(|cs| cs == &"TLS_DHE_RSA_WITH_AES_128_GCM_SHA256"));
    /// ```
    pub fn recommended_cs_iana(&self, psk: bool) -> Vec<&'static str> {
        match (self, psk) {
            (Version::V1Draft, false) => V1_DRAFT_RECOMMENDED_CIPHERSUITES.to_vec(),
            (Version::V1Draft, true) => V1_DRAFT_RECOMMENDED_CIPHERSUITES
                .iter()
                .chain(V1_DRAFT_RECOMMENDED_CIPHERSUITES_PSK_ONLY)
                .copied()
                .collect(),
        }
    }

    /// Returns a [`Vec`] of the suggested ciphersuites for TLS,
    /// according to `self` version. The ciphersuites are specified in OpenSSL's
    /// format.
    ///
    /// # Examples:
    ///
    /// ```
    /// use fefix::fixs::Version;
    ///
    /// let version = Version::V1Draft;
    /// let ciphersuites_openssl = version.recommended_cs_openssl(false);
    /// assert!(ciphersuites_openssl.iter().any(|cs| cs == &"DHE-RSA-AES128-GCM-SHA256"));
    /// ```
    ///
    /// List all ciphersuites in a colon-separated format, like required by
    /// [`openssl-ciphers`](https://www.openssl.org/docs/manmaster/man1/openssl-ciphers.html).
    ///
    /// ```
    /// use fefix::fixs::Version;
    ///
    /// let version = Version::V1Draft;
    /// let ciphersuites_openssl = version.recommended_cs_openssl(false);
    /// let cipherlist = ciphersuites_openssl.join(":");
    /// println!("Supported ciphers: {}", cipherlist);
    /// ```
    pub fn recommended_cs_openssl(&self, psk: bool) -> Vec<&'static str> {
        self.recommended_cs_iana(psk)
            .iter()
            .map(|s| *IANA_TO_OPENSSL.get(s).unwrap())
            .collect()
    }

    /// Creates an [`SslConnectorBuilder`] with fhe FIXS recommended settings.
    pub fn recommended_connector_builder(&self) -> SslConnectorBuilder {
        let mut context = SslConnector::builder(SslMethod::tls()).unwrap();
        match self {
            Version::V1Draft => {
                context
                    .set_min_proto_version(Some(SslVersion::TLS1_1))
                    .unwrap();
                context
                    .set_max_proto_version(Some(SslVersion::TLS1_2))
                    .unwrap();
                context.set_options(SslOptions::NO_COMPRESSION);
                context.set_options(SslOptions::NO_SESSION_RESUMPTION_ON_RENEGOTIATION);
                context.set_options(SslOptions::NO_TLSV1_3);
            }
        };
        context.set_session_cache_mode(SslSessionCacheMode::SERVER);
        context
            .set_cipher_list(self.recommended_cs_openssl(false).join(":").as_str())
            .unwrap();
        context
    }

    /// Creates an [`SslacceptorBuilder`] with fhe FIXS recommended settings.
    pub fn recommended_acceptor_builder(&self) -> SslAcceptorBuilder {
        let mut context = SslAcceptor::mozilla_intermediate_v5(SslMethod::tls()).unwrap();
        match self {
            Version::V1Draft => {
                context
                    .set_min_proto_version(Some(SslVersion::TLS1_1))
                    .unwrap();
                context
                    .set_max_proto_version(Some(SslVersion::TLS1_2))
                    .unwrap();
                context.set_session_cache_mode(SslSessionCacheMode::SERVER);
                context.set_options(SslOptions::CIPHER_SERVER_PREFERENCE);
                context.set_options(SslOptions::NO_COMPRESSION);
                context.set_options(SslOptions::NO_SESSION_RESUMPTION_ON_RENEGOTIATION);
                context.set_options(SslOptions::NO_TLSV1_3);
            }
        };
        context
            .set_cipher_list(self.recommended_cs_openssl(false).join(":").as_str())
            .unwrap();
        context
    }
}

const V1_DRAFT_RECOMMENDED_CIPHERSUITES: &[&str] = &[
    "TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256",
    "TLS_ECDHE_ECDSA_WITH_AES_256_GCM_SHA384",
    "TLS_ECDHE_ECDSA_WITH_AES_128_CBC_SHA",
    "TLS_ECDHE_ECDSA_WITH_AES_256_CBC_SHA",
    "TLS_ECDHE_ECDSA_WITH_AES_128_CBC_SHA256",
    "TLS_ECDHE_ECDSA_WITH_AES_256_CBC_SHA384",
    "TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256",
    "TLS_DHE_RSA_WITH_AES_128_GCM_SHA256",
    "TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384",
    "TLS_DHE_RSA_WITH_AES_256_GCM_SHA384",
    "TLS_ECDHE_RSA_WITH_AES_128_CBC_SHA",
    "TLS_DHE_RSA_WITH_AES_128_CBC_SHA",
    "TLS_ECDHE_RSA_WITH_AES_256_CBC_SHA",
    "TLS_DHE_RSA_WITH_AES_256_CBC_SHA",
    "TLS_ECDHE_RSA_WITH_AES_128_CBC_SHA256",
    "TLS_DHE_RSA_WITH_AES_128_CBC_SHA256",
    "TLS_DHE_RSA_WITH_AES_256_CBC_SHA256",
    "TLS_ECDHE_RSA_WITH_AES_256_CBC_SHA384",
];

const V1_DRAFT_RECOMMENDED_CIPHERSUITES_PSK_ONLY: &[&str] = &[
    "TLS_DHE_PSK_WITH_AES_128_GCM_SHA256",
    "TLS_DHE_PSK_WITH_AES_256_GCM_SHA384",
    "TLS_ECDHE_PSK_WITH_AES_128_CBC_SHA",
    "TLS_DHE_PSK_WITH_AES_128_CBC_SHA",
    "TLS_ECDHE_PSK_WITH_AES_256_CBC_SHA",
    "TLS_DHE_PSK_WITH_AES_256_CBC_SHA",
    "TLS_ECDHE_PSK_WITH_AES_128_CBC_SHA256",
    "TLS_DHE_PSK_WITH_AES_128_CBC_SHA256",
    "TLS_ECDHE_PSK_WITH_AES_256_CBC_SHA384",
    "TLS_DHE_PSK_WITH_AES_256_CBC_SHA384",
];

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn v1draft_acceptor_is_ok() {
        Version::V1Draft.recommended_acceptor_builder();
    }

    #[test]
    fn v1draft_connector_is_ok() {
        Version::V1Draft.recommended_connector_builder();
    }
}
