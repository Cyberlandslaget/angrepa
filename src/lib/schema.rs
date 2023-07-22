// @generated automatically by Diesel CLI.

diesel::table! {
    exploits (id) {
        id -> Text,
        running -> Bool,
        attack_target -> Nullable<Text>,
        blacklist -> Array<Nullable<Text>>,
        docker_image -> Text,
        exploit_kind -> Text,
    }
}

diesel::table! {
    flags (flag) {
        flag -> Text,
        tick -> Nullable<Int4>,
        stamp -> Nullable<Timestamp>,
        exploit_id -> Nullable<Text>,
        target_ip -> Nullable<Text>,
        flagstore -> Nullable<Text>,
        sent -> Bool,
        status -> Nullable<Text>,
    }
}

diesel::table! {
    runlogs (id) {
        id -> Int4,
        from_exploit_id -> Text,
        from_ip -> Text,
        tick -> Int4,
        stamp -> Timestamp,
        content -> Text,
    }
}

diesel::allow_tables_to_appear_in_same_query!(exploits, flags, runlogs,);
