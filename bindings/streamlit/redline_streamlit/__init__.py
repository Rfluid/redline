"""redline for Streamlit: a comment button per widget, with `{kind: "app", key}` anchors.

    import redline_streamlit as rl
    rl.setup()                      # reads REDLINE_EMBED (the `embed` from register_artifact)
    st.metric("MRR", "$42k")
    rl.comment("mrr", label="MRR metric")

Comments go to the agent session that registered the artifact, like any redline comment.
Without setup (no REDLINE_EMBED), `comment()` renders nothing, so the calls can stay in the app.
"""

from __future__ import annotations

import os
import re
from dataclasses import dataclass
from pathlib import Path

import streamlit.components.v1 as components

__all__ = ["setup", "comment", "Config"]

_FRONTEND = Path(__file__).parent / "frontend"
_component = components.declare_component("redline_comment", path=str(_FRONTEND))
_SRC = re.compile(r'src="(?P<server>https?://[^"/]+)/client\.js"')
_ATTR = re.compile(r'data-redline-(?P<name>artifact|token)="(?P<value>[^"]*)"')


@dataclass(frozen=True)
class Config:
    server: str
    artifact: str
    token: str


_config: Config | None = None


def parse_embed(embed: str) -> Config:
    """Reads server, artifact id and token out of the `embed` snippet."""
    src = _SRC.search(embed)
    attrs = {m["name"]: m["value"] for m in _ATTR.finditer(embed)}
    if not src or "artifact" not in attrs or "token" not in attrs:
        raise ValueError("not a redline embed snippet (expected client.js with data-redline-artifact/token)")
    return Config(server=src["server"], artifact=attrs["artifact"], token=attrs["token"])


def setup(embed: str | None = None, *, artifact: str | None = None, token: str | None = None,
          server: str | None = None) -> Config | None:
    """Configures the artifact to comment on.

    Takes the `embed` snippet, or `artifact` + `token` (+ `server`), or the environment:
    `REDLINE_EMBED`, else `REDLINE_ARTIFACT` + `REDLINE_TOKEN` (+ `REDLINE_SERVER`).
    Prefer the environment: then no token ends up in the app's source.
    """
    global _config
    embed = embed or os.environ.get("REDLINE_EMBED")
    if embed:
        _config = parse_embed(embed)
    else:
        artifact = artifact or os.environ.get("REDLINE_ARTIFACT")
        token = token or os.environ.get("REDLINE_TOKEN")
        server = server or os.environ.get("REDLINE_SERVER", "http://127.0.0.1:7465")
        _config = Config(server=server, artifact=artifact, token=token) if artifact and token else None
    return _config


def comment(key: str, label: str | None = None) -> None:
    """A comment button for the widget identified by `key` (stable across reruns).

    The anchor the agent receives is `{"kind": "app", "key": key, "label": label}`.
    """
    if _config is None:
        return
    _component(server=_config.server, artifact=_config.artifact, token=_config.token,
               anchor_key=key, label=label or key, key=f"redline:{key}", default=None)
