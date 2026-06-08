mod admin;
mod attestation;
mod delivery;
mod device;
mod issuer;
mod request;
mod source;
mod sync;
mod user;

pub use admin::{
    AdminApnsRelaySettings, AdminAppleAppAttestSettings, AdminCounts,
    AdminDeviceAttestationSettings, AdminSettings, AdminSummary,
};
pub use attestation::{
    DeviceAttestationRecord, DeviceAttestationStatus, DeviceAttestationSummary,
    FailedDeviceAttestation, VerifiedDeviceAttestation,
};
pub use delivery::{NotificationDelivery, NotificationDeliveryMode};
pub use device::{
    AdminDevice, CurrentUserResponse, Device, DeviceAttestationRequest, DevicePlatform,
    DeviceSigningKeyRequest, EnrollDeviceRequest, EnrollDeviceResponse,
    UpdateDevicePreferencesRequest, UpdatePushTokenRequest, UpdateUserDeviceRequest, UserDevice,
};
pub use issuer::{
    AdminIssuerToken, CreateIssuerTokenRequest, CreateIssuerTokenResponse, IssuerToken,
};
pub use request::{
    CardField, CardLink, CreateDecisionRequest, CreatedDecisionRequest, Decision, DecisionRequest,
    DecisionResolution, DecisionSignatureRecord, OptionKind, RequestNotification, RequestOption,
    RequestStatus, SubmitDecisionRequest, SubmitDecisionSignature, UserDecision,
};
pub use source::{CreateSourceRequest, Source};
pub use sync::SyncEnvelope;
pub use user::{
    AdminUser, AdminUserSubscriptionUpdate, CreateEnrollmentCodeRequest, CreateUserRequest,
    EnrollmentCodeResponse, UpdateSubscriptionRequest, UpdateUserRequest,
    UpdateUserSubscriptionsRequest, User,
};

fn default_signature_algorithm() -> String {
    crate::signing::DEFAULT_ALGORITHM.to_string()
}
