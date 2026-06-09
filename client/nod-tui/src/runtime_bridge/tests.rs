use anyhow::anyhow;
use async_trait::async_trait;
use nod_client_core::{
    models::{ClientState, Request, UserDevice},
    EnrollParams, NotificationPreferenceParams, RenameDeviceParams, RevokeDeviceParams,
    SelectRequestParams, SelectServerParams, SetSubscriptionParams, ChannelParams,
    SubmitOptionParams,
};

use super::{execute_runtime_command, RuntimeCommand, RuntimeCommandOutcome, RuntimePort};
use crate::test_support::{client_state, request, user_device};

#[derive(Debug, Default)]
struct FakeRuntime {
    calls: Vec<&'static str>,
    fail_submit: bool,
}

#[async_trait]
impl RuntimePort for FakeRuntime {
    async fn enroll(&mut self, _params: EnrollParams) -> anyhow::Result<ClientState> {
        self.calls.push("enroll");
        Ok(client_state())
    }

    async fn refresh(&mut self) -> anyhow::Result<ClientState> {
        self.calls.push("refresh");
        Ok(client_state())
    }

    async fn connect_sync(&mut self) -> anyhow::Result<()> {
        self.calls.push("connect_sync");
        Ok(())
    }

    async fn select_server(&mut self, _params: SelectServerParams) -> anyhow::Result<ClientState> {
        self.calls.push("select_server");
        Ok(client_state())
    }

    async fn forget_server(&mut self, _params: SelectServerParams) -> anyhow::Result<ClientState> {
        self.calls.push("forget_server");
        Ok(client_state())
    }

    async fn select_channel(&mut self, _params: ChannelParams) -> anyhow::Result<ClientState> {
        self.calls.push("select_channel");
        Ok(client_state())
    }

    async fn select_request(
        &mut self,
        _params: SelectRequestParams,
    ) -> anyhow::Result<ClientState> {
        self.calls.push("select_request");
        Ok(client_state())
    }

    async fn submit_option(&mut self, _params: SubmitOptionParams) -> anyhow::Result<Request> {
        self.calls.push("submit_option");
        if self.fail_submit {
            return Err(anyhow!("stale request"));
        }
        Ok(request("deploy", "default"))
    }

    async fn clear_channel(&mut self, _params: ChannelParams) -> anyhow::Result<ClientState> {
        self.calls.push("clear_channel");
        Ok(client_state())
    }

    async fn set_subscription(
        &mut self,
        _params: SetSubscriptionParams,
    ) -> anyhow::Result<ClientState> {
        self.calls.push("set_subscription");
        Ok(client_state())
    }

    async fn set_notification_preference(
        &mut self,
        _params: NotificationPreferenceParams,
    ) -> anyhow::Result<ClientState> {
        self.calls.push("set_notification_preference");
        Ok(client_state())
    }

    async fn list_devices(&mut self) -> anyhow::Result<Vec<UserDevice>> {
        self.calls.push("list_devices");
        Ok(vec![user_device("phone")])
    }

    async fn rename_device(&mut self, _params: RenameDeviceParams) -> anyhow::Result<UserDevice> {
        self.calls.push("rename_device");
        Ok(user_device("renamed"))
    }

    async fn revoke_device(&mut self, _params: RevokeDeviceParams) -> anyhow::Result<ClientState> {
        self.calls.push("revoke_device");
        Ok(client_state())
    }
}

#[tokio::test]
async fn enroll_connects_sync_after_success() {
    let mut runtime = FakeRuntime::default();

    let outcome = execute_runtime_command(
        &mut runtime,
        RuntimeCommand::Enroll(EnrollParams {
            base_url: "http://localhost:8767".to_string(),
            device_name: "terminal".to_string(),
            code: "ABCDEFGH".to_string(),
            notification_sound: Some("default".to_string()),
            platform: None,
        }),
    )
    .await;

    assert!(matches!(outcome, Ok(RuntimeCommandOutcome::State(_))));
    assert_eq!(runtime.calls, vec!["enroll", "connect_sync"]);
}

#[tokio::test]
async fn submit_option_surfaces_runtime_errors() {
    let mut runtime = FakeRuntime {
        fail_submit: true,
        ..FakeRuntime::default()
    };

    let outcome = execute_runtime_command(
        &mut runtime,
        RuntimeCommand::SubmitOption(SubmitOptionParams {
            request_id: "missing".to_string(),
            option_id: "approve".to_string(),
            text: None,
        }),
    )
    .await;

    assert_eq!(
        outcome.err().map(|error| error.to_string()),
        Some("stale request".to_string())
    );
    assert_eq!(runtime.calls, vec!["submit_option"]);
}
