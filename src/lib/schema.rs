// @generated automatically by Diesel CLI.

diesel::table! {
    execution (id) {
        id -> Int4,
        exploit_id -> Int4,
        output -> Text,
        started_at -> Timestamp,
        finished_at -> Timestamp,
        target_id -> Int4,
    }
}

diesel::table! {
    exploit (id) {
        id -> Int4,
        name -> Text,
        service -> Text,
        blacklist -> Array<Nullable<Text>>,
        enabled -> Bool,
        docker_image -> Text,
        docker_containers -> Array<Nullable<Text>>,
        pool_size -> Int4,
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

diesel::table! {
    service (name) {
        name -> Text,
    }
}

diesel::table! {
    target (id) {
        id -> Int4,
        flag_id -> Text,
        exploited -> Bool,
        service -> Text,
        team -> Text,
    }
}

diesel::table! {
    team (ip) {
        ip -> Text,
        name -> Nullable<Text>,
    }
}

diesel::joinable!(execution -> exploit (exploit_id));
diesel::joinable!(execution -> target (target_id));
diesel::joinable!(exploit -> service (service));
diesel::joinable!(flag -> execution (execution_id));
diesel::joinable!(flag -> exploit (exploit_id));
diesel::joinable!(target -> service (service));
diesel::joinable!(target -> team (team));

diesel::allow_tables_to_appear_in_same_query!(execution, exploit, flag, service, target, team,);
