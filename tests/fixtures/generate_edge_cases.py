#!/usr/bin/env python3
"""Generate edge-case golden JSON fixtures from the official A2A Python SDK.

Tests corner cases, empty collections, nested types, all enum values,
and every SecurityScheme variant with full detail.

Run: python3 tests/fixtures/generate_edge_cases.py
"""

import json
from pathlib import Path

from a2a.types import (
    AgentCapabilities,
    AgentCard,
    AgentCardSignature,
    AgentExtension,
    AgentInterface,
    AgentProvider,
    AgentSkill,
    APIKeySecurityScheme,
    Artifact,
    AuthorizationCodeOAuthFlow,
    ClientCredentialsOAuthFlow,
    DataPart,
    FilePart,
    FileWithBytes,
    FileWithUri,
    HTTPAuthSecurityScheme,
    ImplicitOAuthFlow,
    Message,
    MessageSendConfiguration,
    MessageSendParams,
    MutualTLSSecurityScheme,
    OAuthFlows,
    OAuth2SecurityScheme,
    OpenIdConnectSecurityScheme,
    PasswordOAuthFlow,
    Part,
    PushNotificationAuthenticationInfo,
    PushNotificationConfig,
    Task,
    TaskArtifactUpdateEvent,
    TaskPushNotificationConfig,
    TaskState,
    TaskStatus,
    TaskStatusUpdateEvent,
    TextPart,
)

OUT_DIR = Path(__file__).parent


def dump(name: str, obj):
    if hasattr(obj, "model_dump"):
        data = obj.model_dump(mode="json", exclude_none=True)
    elif isinstance(obj, list):
        data = [
            item.model_dump(mode="json", exclude_none=True) if hasattr(item, "model_dump") else item
            for item in obj
        ]
    else:
        data = obj
    path = OUT_DIR / f"{name}.json"
    with open(path, "w") as f:
        json.dump(data, f, indent=2, sort_keys=True)
    print(f"  {name}.json")


def main():
    print("Generating edge-case fixtures...")

    # -- Message with all Part types mixed --
    dump("message_mixed_parts", Message(
        messageId="msg-mixed",
        role="agent",
        parts=[
            TextPart(text="Here's the file:"),
            FilePart(file=FileWithBytes(bytes="AQID", mimeType="application/octet-stream")),
            FilePart(file=FileWithUri(uri="https://example.com/data.csv", mimeType="text/csv", name="data.csv")),
            DataPart(data={"summary": {"total": 100, "passed": 95}}),
        ],
        metadata={"turn": 3},
    ))

    # -- Task with all terminal states --
    for state in [TaskState.completed, TaskState.failed, TaskState.canceled, TaskState.rejected]:
        dump(f"task_state_{state.value.replace('-', '_')}", Task(
            id=f"task-{state.value}",
            contextId="ctx-states",
            status=TaskStatus(state=state),
        ))

    # -- Task with input-required (interrupted state) --
    dump("task_state_input_required", Task(
        id="task-input-required",
        contextId="ctx-states",
        status=TaskStatus(
            state=TaskState.input_required,
            message=Message(
                messageId="m-ir",
                role="agent",
                parts=[TextPart(text="Please provide your API key")],
            ),
        ),
    ))

    # -- Task with auth-required --
    dump("task_state_auth_required", Task(
        id="task-auth-required",
        contextId="ctx-states",
        status=TaskStatus(state=TaskState.auth_required),
    ))

    # -- Task with empty artifacts and history lists --
    dump("task_empty_collections", Task(
        id="task-empty",
        contextId="ctx-empty",
        status=TaskStatus(state=TaskState.submitted),
        artifacts=[],
        history=[],
    ))

    # -- Task with multiple artifacts --
    dump("task_multiple_artifacts", Task(
        id="task-multi-art",
        contextId="ctx-multi",
        status=TaskStatus(state=TaskState.completed),
        artifacts=[
            Artifact(artifactId="a1", parts=[TextPart(text="First output")]),
            Artifact(artifactId="a2", parts=[DataPart(data={"result": 42})]),
            Artifact(
                artifactId="a3",
                name="combined",
                description="All results combined",
                parts=[
                    TextPart(text="Summary"),
                    DataPart(data={"items": [1, 2, 3]}),
                ],
                extensions=["urn:ext:v1"],
                metadata={"format": "combined"},
            ),
        ],
    ))

    # -- Artifact with file parts --
    dump("artifact_file_parts", Artifact(
        artifactId="art-files",
        parts=[
            FilePart(file=FileWithBytes(bytes="cHl0aG9u", mimeType="text/x-python", name="script.py")),
            FilePart(file=FileWithUri(uri="https://cdn.example.com/image.png", mimeType="image/png")),
        ],
        name="code_and_image",
    ))

    # -- TaskStatusUpdateEvent with message in status --
    dump("status_update_with_message", TaskStatusUpdateEvent(
        taskId="t1",
        contextId="c1",
        status=TaskStatus(
            state=TaskState.input_required,
            message=Message(
                messageId="m-prompt",
                role="agent",
                parts=[TextPart(text="I need more information")],
            ),
            timestamp="2024-06-15T14:30:00Z",
        ),
        final=False,
    ))

    # -- TaskArtifactUpdateEvent with append --
    dump("artifact_update_append", TaskArtifactUpdateEvent(
        taskId="t1",
        contextId="c1",
        artifact=Artifact(
            artifactId="streaming-art",
            parts=[TextPart(text="...more content...")],
        ),
        append=True,
        lastChunk=False,
    ))

    # -- SecurityScheme: API key in all locations --
    dump("security_apikey_query", APIKeySecurityScheme(
        in_="query",
        name="api_key",
        description="API key passed as query parameter",
    ))
    dump("security_apikey_cookie", APIKeySecurityScheme(
        in_="cookie",
        name="session_id",
    ))

    # -- SecurityScheme: HTTP with various schemes --
    dump("security_http_basic", HTTPAuthSecurityScheme(
        scheme="basic",
        description="HTTP Basic authentication",
    ))
    dump("security_http_bearer_no_format", HTTPAuthSecurityScheme(
        scheme="bearer",
    ))

    # -- SecurityScheme: OAuth2 with client_credentials --
    dump("security_oauth2_client_creds", OAuth2SecurityScheme(
        flows=OAuthFlows(
            clientCredentials=ClientCredentialsOAuthFlow(
                tokenUrl="https://auth.example.com/oauth/token",
                scopes={"api:read": "Read API", "api:write": "Write API"},
            )
        ),
        description="Service-to-service auth",
    ))

    # -- SecurityScheme: OAuth2 with multiple flows --
    dump("security_oauth2_multi_flow", OAuth2SecurityScheme(
        flows=OAuthFlows(
            authorizationCode=AuthorizationCodeOAuthFlow(
                authorizationUrl="https://auth.example.com/authorize",
                tokenUrl="https://auth.example.com/token",
                refreshUrl="https://auth.example.com/refresh",
                scopes={"read": "Read", "write": "Write"},
            ),
            clientCredentials=ClientCredentialsOAuthFlow(
                tokenUrl="https://auth.example.com/token",
                scopes={"admin": "Admin"},
            ),
        ),
        oauth2MetadataUrl="https://auth.example.com/.well-known/oauth-authorization-server",
    ))

    # -- SecurityScheme: OpenID Connect --
    dump("security_openid_with_desc", OpenIdConnectSecurityScheme(
        openIdConnectUrl="https://accounts.google.com/.well-known/openid-configuration",
        description="Google OIDC",
    ))

    # -- SecurityScheme: mTLS with description --
    dump("security_mtls_with_desc", MutualTLSSecurityScheme(
        description="Client certificate required",
    ))

    # -- AgentCard with security schemes --
    dump("agent_card_with_security", AgentCard(
        name="Secure Agent",
        description="An agent with security",
        version="2.0.0",
        url="https://secure-agent.example.com",
        capabilities=AgentCapabilities(streaming=True),
        defaultInputModes=["text/plain", "application/json"],
        defaultOutputModes=["text/plain"],
        skills=[AgentSkill(
            id="secure-op",
            name="Secure Operation",
            description="Does secure things",
            tags=["security"],
        )],
        securitySchemes={
            "bearer": HTTPAuthSecurityScheme(scheme="bearer", bearerFormat="JWT"),
            "apikey": APIKeySecurityScheme(in_="header", name="X-API-Key"),
        },
        security=[{"bearer": []}, {"apikey": []}],
        provider=AgentProvider(
            organization="SecureCorp",
            url="https://securecorp.example.com",
        ),
    ))

    # -- AgentCard with extensions --
    dump("agent_card_with_extensions", AgentCard(
        name="Extended Agent",
        description="Agent with protocol extensions",
        version="1.0.0",
        url="https://extended.example.com",
        capabilities=AgentCapabilities(
            streaming=True,
            pushNotifications=True,
            extensions=[
                AgentExtension(
                    uri="urn:a2a:ext:custom",
                    description="Custom extension",
                    required=True,
                    params={"maxTokens": 4096},
                ),
            ],
        ),
        defaultInputModes=["text/plain"],
        defaultOutputModes=["text/plain"],
        skills=[AgentSkill(
            id="ext-skill",
            name="Extended Skill",
            description="Uses extensions",
            tags=["extended"],
            examples=["Do extended thing"],
            inputModes=["text/plain", "image/png"],
            outputModes=["application/json"],
        )],
    ))

    # -- AgentCard with signatures --
    dump("agent_card_with_signatures", AgentCard(
        name="Signed Agent",
        description="Agent with JWS signatures",
        version="1.0.0",
        url="https://signed.example.com",
        capabilities=AgentCapabilities(),
        defaultInputModes=["text/plain"],
        defaultOutputModes=["text/plain"],
        skills=[AgentSkill(id="s1", name="Skill", description="A skill", tags=["test"])],
        signatures=[
            AgentCardSignature(
                protected="eyJhbGciOiJSUzI1NiJ9",
                signature="dGVzdC1zaWduYXR1cmU",
                header={"kid": "key-001"},
            ),
        ],
    ))

    # -- PushNotificationConfig with all fields --
    dump("push_config_full", PushNotificationConfig(
        id="pn-full",
        url="https://hooks.example.com/a2a",
        token="verify-me",
        authentication=PushNotificationAuthenticationInfo(
            schemes=["Bearer", "Basic"],
            credentials="multi-scheme-token",
        ),
    ))

    # -- MessageSendParams with everything --
    dump("send_params_full", MessageSendParams(
        message=Message(
            messageId="m-full",
            role="user",
            parts=[
                TextPart(text="Process this file"),
                FilePart(file=FileWithUri(uri="https://example.com/input.json", mimeType="application/json")),
            ],
            contextId="ctx-existing",
            taskId="task-existing",
            extensions=["urn:a2a:ext:streaming"],
            referenceTaskIds=["task-prev-1", "task-prev-2"],
            metadata={"priority": "high", "source": "api"},
        ),
        configuration=MessageSendConfiguration(
            acceptedOutputModes=["text/plain", "application/json", "image/png"],
            historyLength=20,
            blocking=False,
            pushNotificationConfig=PushNotificationConfig(
                url="https://hooks.example.com/updates",
                authentication=PushNotificationAuthenticationInfo(
                    schemes=["Bearer"],
                    credentials="webhook-secret",
                ),
            ),
        ),
        metadata={"request_id": "req-123"},
    ))

    # -- Data part with nested complex JSON --
    dump("part_data_complex", DataPart(
        data={
            "array": [1, "two", 3.0, True, None, {"nested": "obj"}],
            "nested": {"deep": {"deeper": {"deepest": "value"}}},
            "empty_obj": {},
            "empty_array": [],
        },
        metadata={"schema_version": 2},
    ))

    # -- Message with empty parts list (edge case) --
    dump("message_empty_parts", Message(
        messageId="msg-empty",
        role="user",
        parts=[],
    ))

    print(f"\nDone! Generated {len(list(OUT_DIR.glob('*.json')))} total fixture files.")


if __name__ == "__main__":
    main()
