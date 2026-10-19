"""Example: a small dashboard with a redline comment button per widget.

    REDLINE_EMBED="$(redline register 'Pricing review' --session S | jq -r .embed)" \
      PYTHONPATH=bindings/streamlit streamlit run bindings/streamlit/example_app.py
"""

import streamlit as st

import redline_streamlit as rl

rl.setup()

st.title("Pricing review")

left, right = st.columns([6, 1])
with left:
    st.metric("Monthly recurring revenue", "$42k", "+8%")
with right:
    rl.comment("mrr", label="MRR metric")

st.subheader("Plans")
st.table([
    {"plan": "Starter", "price": "$9", "seats": 1},
    {"plan": "Team", "price": "$29", "seats": 5},
    {"plan": "Enterprise", "price": "talk to us", "seats": "∞"},
])
rl.comment("pricing_table", label="Plans table")
