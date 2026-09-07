// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! What can go wrong resolving a coded field, as types a caller can branch on.
//!
//! Nothing here flattens a failure into an empty picker. A refused expansion, a
//! timeout, and a binding the client cannot turn into a request are each an
//! error carrying what the server said (`docs/architecture.md` section 7.1,
//! `.claude/rules/reliability.md`).
//!
//! The status variants cover the codes HL7 FHIR R4 4.0.1 `http.html`
//! section 3.1.0.4.2 lists for a rejected interaction, plus one catch-all,
//! because section 3.1.0.1.5 constrains only "specific HTTP status codes in
//! particular circumstances" and leaves the rest to HTTP itself.

use http::StatusCode;
use thiserror::Error;

use ferrochart_form::ids::{LocalCode, TerminologyName};

/// A failure of one terminology resolution.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum TermError {
    /// The base URL cannot carry a path, so no endpoint can be built from it.
    #[error("the terminology server base URL `{base}` cannot carry a path")]
    OpaqueBase {
        /// The URL as configured.
        base: String,
    },

    /// The request never got an answer: a connection failure, a timeout, or a
    /// body that could not be read.
    #[error("the request to the terminology server failed")]
    Transport {
        /// Why the request failed.
        #[source]
        source: reqwest::Error,
    },

    /// The request itself has no FHIR JSON form.
    ///
    /// A value set supplied with the request can carry a value the codec
    /// cannot write, and sending a truncated request would ask the server a
    /// different question from the one the caller asked.
    #[error("the terminology request has no FHIR JSON form")]
    Request {
        /// Why it could not be written.
        #[source]
        source: fhir_types::codec::EncodeError,
    },

    /// The server answered, and the body is not JSON.
    #[error("the terminology server's response body is not JSON")]
    NotJson {
        /// Why it could not be read.
        #[source]
        source: serde_json::Error,
    },

    /// The server answered JSON, and it is not the FHIR resource the call
    /// reads.
    ///
    /// HL7 FHIR R4 4.0.1 `operations.html` section 3.2.0.6.2: an operation
    /// whose single output parameter is named `return` and is a resource
    /// answers with "the resource itself", and every other operation answers
    /// with a `Parameters` resource.
    #[error("the terminology server's response is not {expected}")]
    Body {
        /// What the call was reading, for example "a `ValueSet`".
        expected: &'static str,
        /// Which element was refused, and why.
        #[source]
        source: fhir_types::codec::DecodeError,
    },

    /// The `Parameters` the server answered do not fit the operation's
    /// declared parameter set.
    ///
    /// An out parameter FHIR R4 does not declare is reported rather than
    /// dropped, because a client that silently ignored it would also silently
    /// ignore one it needed.
    #[error("the terminology server's {operation} response does not fit the operation")]
    OutParameters {
        /// The operation, as `Resource/$code`.
        operation: &'static str,
        /// Which parameter was refused, and why.
        #[source]
        source: fhir_types::operation::ParametersError,
    },

    /// The server does not know the value set the binding names.
    ///
    /// A value set the server has never heard of is a different outcome from a
    /// server that is down, and a form has to tell a clinician which.
    #[error("the terminology server does not know `{target}` ({status})")]
    UnknownValueSet {
        /// The status it answered with.
        status: StatusCode,
        /// The value set the request named.
        target: String,
        /// What the `OperationOutcome` said, where the body carried one.
        diagnostics: Option<String>,
        /// The response body, verbatim.
        body: String,
    },

    /// The request carried no valid credentials, or the ones it carried are
    /// not permitted to do this.
    ///
    /// HL7 FHIR R4 4.0.1 `http.html` section 3.1.0.4.2 gives 401 as
    /// "authorization is required for the interaction that was attempted". The
    /// specification mandates no scheme, so the challenge the server sent is
    /// carried rather than interpreted.
    #[error("the terminology server refused the request with {status}")]
    Unauthorized {
        /// 401, 403 or 407.
        status: StatusCode,
        /// The `WWW-Authenticate` or `Proxy-Authenticate` challenge, where the
        /// server sent one.
        challenge: Option<String>,
    },

    /// The server does not offer this operation, or does not offer it this
    /// way.
    ///
    /// A conformant FHIR server need not implement every terminology
    /// operation, so this is a capability answer rather than a defect.
    #[error("the terminology server does not implement this operation ({status})")]
    Unimplemented {
        /// 404 on the operation endpoint, 405, or 501.
        status: StatusCode,
        /// The response body, verbatim.
        body: String,
    },

    /// The server refused the request as malformed, or refused to answer it.
    ///
    /// HL7 FHIR R4 4.0.1 `valueset-operation-expand.html` section 4.9.15.1
    /// requires an error where the server cannot expand properly, and permits
    /// an `OperationOutcome` with code `too-costly` where the expansion is too
    /// large. `codesystem-operation-lookup.html` section 4.8.21.1 answers an
    /// unknown code with 400 and an `OperationOutcome`.
    #[error("the terminology server refused the request ({status})")]
    Refused {
        /// 400 or 422.
        status: StatusCode,
        /// What the request named.
        target: String,
        /// What the `OperationOutcome` said, where the body carried one.
        diagnostics: Option<String>,
        /// The response body, verbatim.
        body: String,
    },

    /// The server gave up on the request, or failed inside itself.
    ///
    /// Nothing is retried here: a caller that knows what it asked for decides
    /// whether asking again is worth it.
    #[error("the terminology server answered {status}: {body}")]
    Unavailable {
        /// 408, 500, 503 or 504.
        status: StatusCode,
        /// The response body, verbatim.
        body: String,
    },

    /// Any other status the server answered with.
    #[error("the terminology server answered {status}: {body}")]
    Upstream {
        /// The status.
        status: StatusCode,
        /// What the `OperationOutcome` said, where the body carried one.
        diagnostics: Option<String>,
        /// The response body, verbatim.
        body: String,
    },

    /// The binding names a target no FHIR request can be built from.
    ///
    /// The operational templates in `corpus/templates` state a
    /// `C_CODE_REFERENCE.referenceSetUri` in a `terminology:` form that no
    /// openEHR specification defines, so the client reports it rather than
    /// guessing what the author meant.
    #[error("`{uri}` is not a target a FHIR terminology request can be built from")]
    UnresolvableBinding {
        /// The URI the template states, verbatim.
        uri: String,
    },

    /// The value set names several binding targets and the template pins none.
    ///
    /// openEHR AM Release-2.3.0 `ADL2.html` section 8.2 warns that a binding
    /// implies no coverage, so picking one of several targets would be a
    /// guess about which terminology the author meant.
    #[error("the value set `{code}` binds to {} terminologies and the template pins none", terminologies.len())]
    AmbiguousBinding {
        /// The `ac`-code naming the set.
        code: LocalCode,
        /// Every terminology the template binds it to, in name order.
        terminologies: Vec<TerminologyName>,
    },

    /// No FHIR system URI is known for the terminology the template names.
    ///
    /// The correspondence between an openEHR `terminology_id` and a FHIR
    /// system URI is not governed by any specification, so a name the default
    /// table does not carry is reported rather than guessed at. A deployer
    /// states it with
    /// [`TermClient::with_system`](crate::client::TermClient::with_system).
    #[error("no FHIR system URI is configured for the openEHR terminology `{terminology}`")]
    UnknownSystem {
        /// The terminology the template names.
        terminology: TerminologyName,
    },

    /// The form definition states a kind of value set this client does not
    /// resolve.
    ///
    /// `ferrochart_form::value::ValueSet` and its `ExpansionSource` are both
    /// `#[non_exhaustive]`, so a kind added there and not taught here would
    /// otherwise fall through silently. It arrives as an error instead.
    #[error("the form definition states a value set kind this client does not resolve")]
    UnsupportedValueSet,

    /// The field needs a terminology server and none is configured.
    #[error("resolving `{target}` needs a terminology server and none is configured")]
    NoServer {
        /// What would have been asked for.
        target: String,
    },

    /// A panic in another task left the expansion cache locked.
    ///
    /// A poisoned lock is reported rather than read as a cache miss, because a
    /// miss would silently turn a broken process into extra network traffic.
    #[error("the expansion cache is poisoned by a panic in another task")]
    CachePoisoned,
}
