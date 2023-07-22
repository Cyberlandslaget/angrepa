// @generated automatically by Diesel CLI.

diesel::table! {
    execution (id) {
        id -> Int4,
        exploit_id -> Int4,
        output -> Text,
        started_at -> Timestamp,
        finished_at -> Timestamp,
    }
}

diesel::table! {
    exploit (id) {
        id -> Int4,
        name -> Text,
        service -> Text,
        blacklist -> Text,
        docker_image -> Text,
        enabled -> Bool,
    }
}

diesel::table! {
    flag (id) {
        id -> Int4,
        text -> Text,
        status -> Text,
        submitted -> Bool,
        timestamp -> Timestamp,
        execution_id -> Int4,
        exploit_id -> Int4,
    }
}

diesel::joinable!(execution -> exploit (exploit_id));
diesel::joinable!(flag -> execution (execution_id));
diesel::joinable!(flag -> exploit (exploit_id));

diesel::allow_tables_to_appear_in_same_query!(execution, exploit, flag,);
