// Matches `diesel print-schema` for migrations/. Regenerate after adding a migration.

diesel::table! {
    artifacts (id) {
        id -> Text,
        title -> Text,
        uri -> Nullable<Text>,
        kind -> Text,
        session -> Nullable<Text>,
        token -> Text,
        created -> BigInt,
    }
}

diesel::table! {
    threads (id) {
        id -> Text,
        artifact_id -> Text,
        anchor -> Text,
        status -> Text,
        created -> BigInt,
        updated -> BigInt,
        note -> Nullable<Text>,
    }
}

diesel::table! {
    messages (seq) {
        seq -> BigInt,
        id -> Text,
        thread_id -> Text,
        author -> Text,
        body -> Text,
        created -> BigInt,
    }
}

diesel::table! {
    events (seq) {
        seq -> BigInt,
        artifact_id -> Text,
        thread_id -> Text,
        kind -> Text,
        author -> Text,
        at -> BigInt,
    }
}

diesel::table! {
    attachments (id) {
        id -> Text,
        artifact_id -> Text,
        mime -> Text,
        data -> Binary,
        created -> BigInt,
    }
}

diesel::joinable!(threads -> artifacts (artifact_id));
diesel::joinable!(attachments -> artifacts (artifact_id));
diesel::joinable!(messages -> threads (thread_id));
diesel::joinable!(events -> artifacts (artifact_id));

diesel::allow_tables_to_appear_in_same_query!(artifacts, threads, messages, events, attachments);
