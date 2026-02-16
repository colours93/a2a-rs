#!/usr/bin/env python3
"""Generate golden JSON fixtures from the official A2A Python SDK.

These fixtures are the GROUND TRUTH for wire format compliance.
The Rust SDK must deserialize every one of these, and when it
re-serializes, the output must match byte-for-byte (modulo key order).

Run: python3 tests/fixtures/generate_golden.py
Output: tests/fixtures/*.json
"""

import json
import sys
from pathlib import Path

# Import from the official A2A Python SDK
from a2a.types import (
    Artifact,
    AgentCard,
    AgentCapabilities,
    AgentExtension,
    AgentInterface,
    AgentProvider,
    AgentSkill,
    APIKeySecurityScheme,
    AuthorizationCodeOAuthFlow,
    ClientCredentialsOAuthFlow,
    DataPart,
    FilePart,
    FileWithBytes,
    FileWithUri,
    HTTPAuthSecurityScheme,
    Message,
    MessageSendConfiguration,
    MessageSendParams,
    MutualTLSSecurityScheme,
    OAuthFlows,
    OAuth2SecurityScheme,
    OpenIdConnectSecurityScheme,
    Part,
    PushNotificationAuthenticationInfo,
    PushNotificationConfig,
    SecurityScheme,
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
    """Serialize a Pydantic model to JSON and write to file."""
    if hasattr(obj, "model_dump"):
        data = obj.model_dump(mode="json", exclude_none=True)
    elif isinstance(obj, list):
        data = [
            item.model_dump(mode="json", exclude_none=True)
            if hasattr(item, "model_dump")
            else item
            for item in obj
        ]
    else:
        data = obj

    path = OUT_DIR / f"{name}.json"
    with open(path, "w") as f:
        json.dump(data, f, indent=2, sort_keys=True)
    print(f"  {name}.json")


def main():
    print("Generating golden fixtures from A2A Python SDK...")

    # -- TaskState enum values --
    dump("task_state_all", [s.value for s in TaskState])

    # -- TextPart --
    dump("part_text", TextPart(text="Hello, world!"))
    dump("part_text_with_metadata", TextPart(text="Hello", metadata={"source": "test"}))

    # -- FilePart with bytes --
    dump(
        "part_file_bytes",
        FilePart(
            file=FileWithBytes(
                bytes="SGVsbG8gV29ybGQ=",
                mimeType="text/plain",
                name="hello.txt",
            )
        ),
    )

    # -- FilePart with URI --
    dump(
        "part_file_uri",
        FilePart(
            file=FileWithUri(
                uri="https://example.com/doc.pdf",
                mimeType="application/pdf",
            )
        ),
    )

    # -- DataPart --
    dump("part_data", DataPart(data={"key": "value", "count": 42}))
    dump(
        "part_data_with_metadata",
        DataPart(data={"items": [1, 2, 3]}, metadata={"schema": "v1"}),
    )

    # -- Message (minimal) --
    dump(
        "message_minimal",
        Message(
            messageId="msg-001",
            role="user",
            parts=[TextPart(text="Hello agent")],
        ),
    )

    # -- Message (full) --
    dump(
        "message_full",
        Message(
            messageId="msg-002",
            role="agent",
            parts=[TextPart(text="Response")],
            contextId="ctx-1",
            taskId="task-1",
            metadata={"model": "gpt-4"},
            extensions=["urn:a2a:ext:streaming"],
            referenceTaskIds=["task-0"],
        ),
    )

    # -- TaskStatus --
    dump("task_status_minimal", TaskStatus(state=TaskState.working))
    dump(
        "task_status_full",
        TaskStatus(
            state=TaskState.completed,
            message=Message(
                messageId="m1",
                role="agent",
                parts=[TextPart(text="Done!")],
            ),
            timestamp="2024-01-15T10:30:00Z",
        ),
    )

    # -- Task (minimal) --
    dump(
        "task_minimal",
        Task(
            id="task-001",
            contextId="ctx-001",
            status=TaskStatus(state=TaskState.submitted),
        ),
    )

    # -- Task (full) --
    dump(
        "task_full",
        Task(
            id="task-002",
            contextId="ctx-002",
            status=TaskStatus(
                state=TaskState.completed,
                timestamp="2024-01-15T12:00:00Z",
            ),
            artifacts=[
                Artifact(
                    artifactId="art-1",
                    parts=[TextPart(text="Result data")],
                    name="output",
                    description="The output artifact",
                )
            ],
            history=[
                Message(
                    messageId="m1",
                    role="user",
                    parts=[TextPart(text="Do something")],
                )
            ],
            metadata={"priority": "high"},
        ),
    )

    # -- Artifact --
    dump(
        "artifact",
        Artifact(
            artifactId="art-001",
            name="code_output",
            description="Generated code",
            parts=[
                TextPart(text="fn main() {}"),
                DataPart(data={"language": "rust"}),
            ],
            extensions=["urn:a2a:ext:code"],
        ),
    )

    # -- TaskStatusUpdateEvent --
    dump(
        "task_status_update_event",
        TaskStatusUpdateEvent(
            taskId="task-001",
            contextId="ctx-001",
            status=TaskStatus(state=TaskState.working),
            final=False,
        ),
    )
    dump(
        "task_status_update_event_final",
        TaskStatusUpdateEvent(
            taskId="task-002",
            contextId="ctx-002",
            status=TaskStatus(state=TaskState.completed),
            final=True,
        ),
    )

    # -- TaskArtifactUpdateEvent --
    dump(
        "task_artifact_update_event",
        TaskArtifactUpdateEvent(
            taskId="task-001",
            contextId="ctx-001",
            artifact=Artifact(
                artifactId="art-1",
                parts=[TextPart(text="chunk 1")],
            ),
            append=False,
            lastChunk=True,
        ),
    )

    # -- SecurityScheme variants --
    dump(
        "security_scheme_apikey",
        APIKeySecurityScheme(in_="header", name="X-API-Key"),
    )
    dump(
        "security_scheme_http",
        HTTPAuthSecurityScheme(scheme="bearer", bearerFormat="JWT"),
    )
    dump(
        "security_scheme_oauth2",
        OAuth2SecurityScheme(
            flows=OAuthFlows(
                authorizationCode=AuthorizationCodeOAuthFlow(
                    authorizationUrl="https://auth.example.com/authorize",
                    tokenUrl="https://auth.example.com/token",
                    scopes={"read": "Read access", "write": "Write access"},
                )
            )
        ),
    )
    dump(
        "security_scheme_openid",
        OpenIdConnectSecurityScheme(
            openIdConnectUrl="https://auth.example.com/.well-known/openid-configuration"
        ),
    )
    dump("security_scheme_mtls", MutualTLSSecurityScheme())

    # -- AgentInterface --
    dump(
        "agent_interface",
        AgentInterface(
            url="https://api.example.com/a2a",
            transport="JSONRPC",
        ),
    )
    dump(
        "agent_interface_with_version",
        AgentInterface(
            url="https://grpc.example.com/a2a",
            transport="GRPC",
            protocolVersion="0.3",
        ),
    )

    # -- AgentCapabilities --
    dump("agent_capabilities_empty", AgentCapabilities())
    dump(
        "agent_capabilities_full",
        AgentCapabilities(
            streaming=True,
            pushNotifications=False,
            stateTransitionHistory=True,
        ),
    )

    # -- AgentSkill --
    dump(
        "agent_skill",
        AgentSkill(
            id="code-gen",
            name="Code Generation",
            description="Generates code in various languages",
            tags=["coding", "generation"],
            examples=["Write a function", "Generate a class"],
            inputModes=["text/plain"],
            outputModes=["text/plain", "application/json"],
        ),
    )

    # -- AgentProvider --
    dump(
        "agent_provider",
        AgentProvider(organization="Acme Corp", url="https://acme.example.com"),
    )

    # -- PushNotificationConfig --
    dump(
        "push_notification_config",
        PushNotificationConfig(
            id="pn-1",
            url="https://hooks.example.com/notify",
            token="verify-token-123",
            authentication=PushNotificationAuthenticationInfo(
                schemes=["Bearer"],
                credentials="secret-token",
            ),
        ),
    )

    # -- TaskPushNotificationConfig --
    dump(
        "task_push_notification_config",
        TaskPushNotificationConfig(
            id="tpnc-1",
            taskId="task-001",
            pushNotificationConfig=PushNotificationConfig(
                url="https://hooks.example.com/notify",
            ),
        ),
    )

    # -- MessageSendParams --
    dump(
        "send_message_params",
        MessageSendParams(
            message=Message(
                messageId="m1",
                role="user",
                parts=[TextPart(text="Hello")],
            ),
            configuration=MessageSendConfiguration(
                acceptedOutputModes=["text/plain", "application/json"],
                historyLength=10,
                blocking=True,
            ),
        ),
    )

    # -- AgentCard (minimal) --
    dump(
        "agent_card_minimal",
        AgentCard(
            name="Test Agent",
            description="A test agent",
            version="1.0.0",
            url="https://agent.example.com",
            supportedInterfaces=[
                AgentInterface(
                    url="https://agent.example.com/a2a",
                    transport="JSONRPC",
                )
            ],
            capabilities=AgentCapabilities(),
            defaultInputModes=["text/plain"],
            defaultOutputModes=["text/plain"],
            skills=[
                AgentSkill(
                    id="echo",
                    name="Echo",
                    description="Echoes input",
                    tags=["utility"],
                )
            ],
        ),
    )

    print(f"\nDone! Generated {len(list(OUT_DIR.glob('*.json')))} fixture files.")


if __name__ == "__main__":
    main()
