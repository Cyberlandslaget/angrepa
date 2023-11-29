use crate::models::{
    ExecutionInserter, ExploitInserter, ExploitModel, FlagInserter, FlagModel, TargetModel,
};
use crate::types::{Execution, Exploit, Flag, Service, Target, TargetInserter, Team};
use chrono::NaiveDateTime;
use lexical_sort::natural_lexical_cmp;

#[derive(Clone)]
pub struct Db {
    conn: sqlx::Pool<sqlx::Postgres>,
}

impl Db {
    pub fn wrap(conn: sqlx::Pool<sqlx::Postgres>) -> Self {
        Self { conn }
    }

    // == teams ==

    pub async fn teams(&self) -> Result<Vec<Team>, DbError> {
        Ok(sqlx::query_as!(Team, "SELECT * FROM team")
            .fetch_all(&self.conn)
            .await?)
    }

    pub async fn team_by_ip(&self, ip: &str) -> Result<Option<Team>, DbError> {
        Ok(
            sqlx::query_as!(Team, "SELECT * from TEAM WHERE ip = $1", ip)
                .fetch_optional(&self.conn)
                .await?,
        )
    }

    // doesn't verify the team exists
    pub async fn team_set_name(&self, ip: &str, name: &str) -> Result<(), DbError> {
        sqlx::query!("UPDATE team SET name = $1 WHERE ip = $2", name, ip)
            .execute(&self.conn)
            .await?;
        Ok(())
    }

    // ignores conflicts
    pub async fn add_team_checked(&self, ip: &str, name: Option<&str>) -> Result<(), DbError> {
        sqlx::query!(
            "INSERT INTO team (ip, name) VALUES ($1, $2) ON CONFLICT (ip) DO NOTHING",
            ip,
            name
        )
        .execute(&self.conn)
        .await?;
        Ok(())
    }

    // == services ==

    pub async fn services(&self) -> Result<Vec<Service>, DbError> {
        Ok(sqlx::query_as!(Service, "SELECT * FROM service")
            .fetch_all(&self.conn)
            .await?)
    }

    // ignore dupes
    pub async fn add_service_checked(&self, name: &str) -> Result<(), DbError> {
        sqlx::query!(
            "INSERT INTO service (name) VALUES ($1) ON CONFLICT (name) DO NOTHING",
            name
        )
        .execute(&self.conn)
        .await?;
        Ok(())
    }

    pub async fn get_service(&self, name: &str) -> Result<Option<Service>, DbError> {
        Ok(
            sqlx::query_as!(Service, "SELECT * FROM service WHERE name = $1", name)
                .fetch_optional(&self.conn)
                .await?,
        )
    }

    // == target ==

    pub async fn add_target(&self, target: &TargetInserter) -> Result<(), DbError> {
        let TargetInserter {
            flag_id,
            service,
            team,
            created_at,
            target_tick,
        } = target;

        sqlx::query!(
            "INSERT INTO target (flag_id, service, team, created_at, target_tick) VALUES ($1, $2, $3, $4, $5)",
            flag_id, service, team, created_at, target_tick
        ).execute(&self.conn).await?;

        Ok(())
    }

    pub async fn get_latest_nop_target(
        &self,
        nop_ip: &str,
        service: &str,
    ) -> Result<Option<Target>, DbError> {
        Ok(
            sqlx::query_as!(Target, "SELECT * FROM target WHERE team = $1 AND service = $2 ORDER BY created_at DESC LIMIT 1", nop_ip, service)
                .fetch_optional(&self.conn)
                .await?,
        )
    }

    // == exploit ==

    pub async fn exploits(&self) -> Result<Vec<Exploit>, DbError> {
        Ok(sqlx::query_as!(Exploit, "SELECT * FROM exploit")
            .fetch_all(&self.conn)
            .await?)
    }

    pub async fn exploit(&self, id: i32) -> Result<Option<Exploit>, DbError> {
        Ok(
            sqlx::query_as!(Exploit, "SELECT * FROM exploit WHERE id = $1", id)
                .fetch_optional(&self.conn)
                .await?,
        )
    }

    pub async fn exploits_for_service(&self, service: &str) -> Result<Vec<Exploit>, DbError> {
        Ok(
            sqlx::query_as!(Exploit, "SELECT * FROM exploit WHERE service = $1", service)
                .fetch_all(&self.conn)
                .await?,
        )
    }

    // does not check the exploit exists
    pub async fn exploit_flags_since(
        &self,
        exploit: i32,
        since: NaiveDateTime,
    ) -> Result<Vec<Flag>, DbError> {
        Ok(sqlx::query_as!(
            Flag,
            "SELECT * FROM flag WHERE exploit_id = $1 AND timestamp >= $2",
            exploit,
            since,
        )
        .fetch_all(&self.conn)
        .await?)
    }

    pub async fn exploit_edit_config(
        &self,
        id: i32,
        name: String,
        blacklist: &[String],
        pool_size: i32,
    ) -> Result<(), DbError> {
        sqlx::query!(
            "UPDATE exploit SET name=$2, blacklist=$3, pool_size=$4 WHERE id=$1",
            id,
            name,
            blacklist,
            pool_size
        )
        .execute(&self.conn)
        .await?;
        Ok(())
    }

    pub async fn add_exploit(&self, exploit: &ExploitInserter) -> Result<Exploit, DbError> {
        let ExploitInserter {
            name,
            service,
            blacklist,
            enabled,
            docker_image,
            docker_containers,
            pool_size,
        } = exploit;

        Ok(sqlx::query_as!(Exploit, "INSERT INTO exploit (name, service, blacklist, enabled, docker_image, docker_containers, pool_size) VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING *", name, service, &blacklist, enabled, docker_image, &docker_containers, pool_size).fetch_one(&self.conn).await?)
    }

    pub async fn start_exploit(&self, exploit: i32) -> Result<(), DbError> {
        sqlx::query!("UPDATE exploit SET enabled=true WHERE id=$1", exploit)
            .execute(&self.conn)
            .await?;
        Ok(())
    }

    pub async fn stop_exploit(&self, exploit: i32) -> Result<(), DbError> {
        sqlx::query!("UPDATE exploit SET enabled=false WHERE id=$1", exploit)
            .execute(&self.conn)
            .await?;
        Ok(())
    }

    pub async fn set_exploit_docker_containers(
        &self,
        exploit: i32,
        containers: &[String],
    ) -> Result<(), DbError> {
        sqlx::query!(
            "UPDATE exploit SET docker_containers=$2 WHERE id=$1",
            exploit,
            containers
        )
        .execute(&self.conn)
        .await?;
        Ok(())
    }

    // == flags ==
    pub async fn flags_since(&self, since: NaiveDateTime) -> Result<Vec<Flag>, DbError> {
        Ok(
            sqlx::query_as!(Flag, "SELECT * FROM flag WHERE timestamp >= $1", since,)
                .fetch_all(&self.conn)
                .await?,
        )
    }

    pub async fn flags_from_service_since(
        &self,
        service: &str,
        since: NaiveDateTime,
    ) -> Result<Vec<Flag>, DbError> {
        let exploits = self.exploits_for_service(service).await?;
        let exploit_ids: Vec<_> = exploits.iter().map(|e| e.id).collect();

        Ok(sqlx::query_as!(
            Flag,
            "SELECT * FROM flag WHERE timestamp >= $1 AND exploit_id = ANY($2)",
            since,
            &exploit_ids
        )
        .fetch_all(&self.conn)
        .await?)
    }

    pub async fn flags_since_extended(
        &self,
        since: NaiveDateTime,
    ) -> Result<Vec<(Flag, Execution, Target)>, DbError> {
        struct All {
            flag: Flag,
            execution: Execution,
            target: Target,
        }

        Ok(sqlx::query_as!(All, r#"
            SELECT
                (f.id, f.text, f.status, f.submitted, f.timestamp, f.execution_id, f.exploit_id) as "flag!: Flag",
                (e.id, e.exploit_id, e.output, e.exit_code, e.started_at, e.finished_at, e.target_id) as "execution!: Execution",
                (t.id, t.flag_id, t.service, t.team, t.created_at, t.target_tick) as "target!: Target"
            FROM
                flag as f
                INNER JOIN execution as e ON f.execution_id = e.id
                INNER JOIN target as t ON e.target_id = t.id
            WHERE
                f.timestamp >= $1
            "#, since
        )
        .fetch_all(&self.conn)
        .await?
        .into_iter()
        .map(|a| (a.flag, a.execution, a.target))
        .collect())
    }

    pub async fn flags_by_id_extended(
        &self,
        ids: &[i32],
    ) -> Result<Vec<(Flag, Execution, Target)>, DbError> {
        struct All {
            flag: Flag,
            execution: Execution,
            target: Target,
        }

        Ok(sqlx::query_as!(All, r#"
            SELECT
                (f.id, f.text, f.status, f.submitted, f.timestamp, f.execution_id, f.exploit_id) as "flag!: Flag",
                (e.id, e.exploit_id, e.output, e.exit_code, e.started_at, e.finished_at, e.target_id) as "execution!: Execution",
                (t.id, t.flag_id, t.service, t.team, t.created_at, t.target_tick) as "target!: Target"
            FROM
                flag as f
                INNER JOIN execution as e ON f.execution_id = e.id
                INNER JOIN target as t ON e.target_id = t.id
            WHERE
                f.id = ANY($1)
            "#, ids
        )
        .fetch_all(&self.conn)
        .await?
        .into_iter()
        .map(|a| (a.flag, a.execution, a.target))
        .collect())
    }

    // == executions ==

    pub async fn add_execution(&self, execution: &ExecutionInserter) -> Result<Execution, DbError> {
        let ExecutionInserter {
            exploit_id,
            output,
            exit_code,
            started_at,
            finished_at,
            target_id,
        } = execution;
        Ok(
            sqlx::query_as!(Execution, "INSERT INTO execution (exploit_id, output, exit_code, started_at, finished_at, target_id) VALUES ($1, $2, $3, $4, $5, $6) RETURNING *", exploit_id, output, exit_code, started_at, finished_at, target_id)
                .fetch_one(&self.conn)
                .await?,
        )
    }

    pub async fn executions_since(&self, since: NaiveDateTime) -> Result<Vec<Execution>, DbError> {
        Ok(sqlx::query_as!(
            Execution,
            "SELECT * FROM execution WHERE started_at >= $1",
            since,
        )
        .fetch_all(&self.conn)
        .await?)
    }

    pub async fn executions_for_service_since(
        &self,
        service: &str,
        since: NaiveDateTime,
    ) -> Result<Vec<Execution>, DbError> {
        let exploits = self.exploits_for_service(service).await?;
        let exploit_ids: Vec<_> = exploits.iter().map(|e| e.id).collect();

        Ok(sqlx::query_as!(
            Execution,
            "SELECT * FROM execution WHERE started_at >= $1 AND exploit_id = ANY($2)",
            since,
            &exploit_ids
        )
        .fetch_all(&self.conn)
        .await?)
    }

    pub async fn executions_since_extended(
        &self,
        since: NaiveDateTime,
    ) -> Result<Vec<(Execution, Target)>, DbError> {
        struct All {
            execution: Execution,
            target: Target,
        }

        Ok(sqlx::query_as!(All, r#"
            SELECT
                (e.id, e.exploit_id, e.output, e.exit_code, e.started_at, e.finished_at, e.target_id) as "execution!: Execution",
                (t.id, t.flag_id, t.service, t.team, t.created_at, t.target_tick) as "target!: Target"
            FROM
                execution as e
                INNER JOIN target as t ON e.target_id = t.id
            WHERE
                e.started_at >= $1
            "#, since
            ).fetch_all(&self.conn).await?.into_iter().map(|a| (a.execution, a.target)).collect()
        )
    }

    pub async fn executions_by_id_extended(
        &self,
        ids: &[i32],
    ) -> Result<Vec<(Execution, Target)>, DbError> {
        struct All {
            execution: Execution,
            target: Target,
        }

        Ok(sqlx::query_as!(All, r#"
            SELECT
                (e.id, e.exploit_id, e.output, e.exit_code, e.started_at, e.finished_at, e.target_id) as "execution!: Execution",
                (t.id, t.flag_id, t.service, t.team, t.created_at, t.target_tick) as "target!: Target"
            FROM
                execution as e
                INNER JOIN target as t ON e.target_id = t.id
            WHERE
                e.id = ANY($1)
            "#, ids
            ).fetch_all(&self.conn).await?.into_iter().map(|a| (a.execution, a.target)).collect()
        )
    }

    // == targets ==

    pub async fn get_exploitable_targets_updating(
        &self,
        oldest: chrono::NaiveDateTime,
    ) -> Result<Vec<(Vec<TargetModel>, ExploitModel)>, DbError> {
        // to be exploitable a target must
        // 1. not already be exploited by the specific exploit
        //       (but can be exploited by another exploit)
        // 2. have an active exploit pointing to it
        // 3. not be older than the N ticks/seconds where N is the max age of a flag
        //
        // targets will also be sorted by oldest first to prioritize flags that are about to expire

        // ^ i think this is outdated, esp point 2 is wrong

        let active_exploits = self
            .exploits()
            .await?
            .into_iter()
            .filter(|e| e.enabled)
            .collect::<Vec<_>>();

        let relevant_executions = sqlx::query_as!(
            Execution,
            r#"
            SELECT
                *
            FROM
                execution
            WHERE
                finished_at >= $1 AND exploit_id = ANY($2)
            "#,
            oldest,
            &active_exploits.iter().map(|e| e.id).collect::<Vec<_>>()
        )
        .fetch_all(&self.conn)
        .await?;

        let mut target_exploits = Vec::new();
        for exploit in active_exploits {
            // the statement generated from the diesel query:
            // SELECT "target"."id", "target"."flag_id", "target"."service",
            //        "target"."team", "target"."created_at", "target"."target_tick"
            // FROM "target"
            // WHERE ((("target"."id" != ALL($1))
            //  AND ("target"."service" = $2))
            //  AND ("target"."created_at" > $3))
            // ORDER BY "target"."created_at" ASC
            // -- binds: [[], "testservice", 2023-11-29T11:50:46.026072]

            // so i think i did this right:

            let mut targets: Vec<Target> = sqlx::query_as!(
                Target,
                r#"
                SELECT
                    *
                FROM
                    target
                WHERE
                    id != ALL($3)
                    AND service = $1
                    AND created_at >= $2
                ORDER BY
                    created_at ASC
                "#,
                exploit.service,
                oldest,
                &relevant_executions
                    .iter()
                    .map(|e| e.target_id)
                    .collect::<Vec<_>>()
            )
            .fetch_all(&self.conn)
            .await?;

            // sort by ip to make viewing an adminer easier
            targets.sort_by(|a, b| natural_lexical_cmp(&a.team, &b.team));

            target_exploits.push((targets, exploit.clone()));
        }

        todo!();
    }

    // == flags ==

    pub async fn add_flag(&self, flag: &FlagInserter) -> Result<(), DbError> {
        let FlagInserter {
            text,
            status,
            submitted,
            timestamp,
            execution_id,
            exploit_id,
        } = flag;

        sqlx::query!(
            "INSERT INTO flag (text, status, submitted, timestamp, execution_id, exploit_id) VALUES ($1, $2, $3, $4, $5, $6)",
            text, status, submitted, timestamp, execution_id, exploit_id
        ).execute(&self.conn).await?;

        Ok(())
    }

    pub async fn update_flag_status(
        &self,
        search_flag: &str,
        new_status: &str,
    ) -> Result<(), DbError> {
        sqlx::query!(
            "UPDATE flag SET status=$2 WHERE text=$1",
            search_flag,
            new_status
        )
        .execute(&self.conn)
        .await?;
        Ok(())
    }

    pub async fn get_unsubmitted_flags(&self) -> Result<Vec<FlagModel>, DbError> {
        Ok(
            sqlx::query_as!(FlagModel, "SELECT * FROM flag WHERE submitted = false")
                .fetch_all(&self.conn)
                .await?,
        )
    }

    pub async fn set_flag_submitted(&self, flag: i32) -> Result<(), DbError> {
        sqlx::query!("UPDATE flag SET submitted=true WHERE id=$1", flag)
            .execute(&self.conn)
            .await?;
        Ok(())
    }
}

#[derive(thiserror::Error, Debug)]
pub enum DbError {
    #[error("sqlx error")]
    Sqlx(#[from] sqlx::Error),
}
