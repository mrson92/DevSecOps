use chrono::{Utc, NaiveDate, Duration};
use sqlx::SqlitePool;
use tracing::info;
use uuid::Uuid;

use aads_core::error::AppError;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
pub struct Report {
    pub id: String,
    #[sqlx(rename = "type")]
    pub report_type: String,
    pub title: String,
    pub period_start: String,
    pub period_end: String,
    pub content: String,
    pub summary: Option<String>,
    pub format: String,
    pub status: String,
    pub generated_at: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ReportContent {
    pub period: PeriodInfo,
    pub summary: ReportSummary,
    pub top_rules: Vec<RuleStat>,
    pub top_ips: Vec<IpStat>,
    pub severity_breakdown: SeverityBreakdown,
    pub hourly_distribution: Vec<u32>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PeriodInfo {
    pub start: String,
    pub end: String,
    pub report_type: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ReportSummary {
    pub total_detections: i64,
    pub open_detections: i64,
    pub resolved_detections: i64,
    pub critical_count: i64,
    pub high_count: i64,
    pub medium_count: i64,
    pub low_count: i64,
    pub unique_rules_triggered: i64,
    pub unique_ips: i64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RuleStat {
    pub rule_id: String,
    pub rule_name: String,
    pub severity: String,
    pub detection_count: i64,
    pub total_matched: i64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IpStat {
    pub ip: String,
    pub detection_count: i64,
    pub rules_triggered: i64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SeverityBreakdown {
    pub critical: i64,
    pub high: i64,
    pub medium: i64,
    pub low: i64,
}

pub struct ReportGenerator {
    db: SqlitePool,
}

impl ReportGenerator {
    pub fn new(db: SqlitePool) -> Self {
        Self { db }
    }

    pub async fn generate_daily_report(&self, date: NaiveDate) -> Result<Report, AppError> {
        let start = format!("{}T00:00:00", date);
        let end = format!("{}T23:59:59", date);
        let title = format!("일일 보고서 - {}", date.format("%Y-%m-%d"));

        self.generate_report("daily", &title, &start, &end).await
    }

    pub async fn generate_weekly_report(&self, start_date: NaiveDate) -> Result<Report, AppError> {
        let end_date = start_date + Duration::days(6);
        let start = format!("{}T00:00:00", start_date);
        let end = format!("{}T23:59:59", end_date);
        let title = format!("주간 보고서 - {} ~ {}", start_date.format("%Y-%m-%d"), end_date.format("%Y-%m-%d"));

        self.generate_report("weekly", &title, &start, &end).await
    }

    pub async fn generate_monthly_report(&self, year: i32, month: u32) -> Result<Report, AppError> {
        let start_date = NaiveDate::from_ymd_opt(year, month, 1).unwrap();
        let end_date = if month == 12 {
            NaiveDate::from_ymd_opt(year + 1, 1, 1).unwrap() - Duration::days(1)
        } else {
            NaiveDate::from_ymd_opt(year, month + 1, 1).unwrap() - Duration::days(1)
        };

        let start = format!("{}T00:00:00", start_date);
        let end = format!("{}T23:59:59", end_date);
        let title = format!("월간 보고서 - {}년 {}월", year, month);

        self.generate_report("monthly", &title, &start, &end).await
    }

    async fn generate_report(
        &self,
        report_type: &str,
        title: &str,
        start: &str,
        end: &str,
    ) -> Result<Report, AppError> {
        let summary = self.fetch_summary(start, end).await?;
        let top_rules = self.fetch_top_rules(start, end, 10).await?;
        let top_ips = self.fetch_top_ips(start, end, 10).await?;
        let severity = self.fetch_severity_breakdown(start, end).await?;
        let hourly = self.fetch_hourly_distribution(start, end).await?;

        let content = ReportContent {
            period: PeriodInfo {
                start: start.to_string(),
                end: end.to_string(),
                report_type: report_type.to_string(),
            },
            summary,
            top_rules,
            top_ips,
            severity_breakdown: severity,
            hourly_distribution: hourly,
        };

        let content_json = serde_json::to_string_pretty(&content).unwrap_or_default();
        let summary_text = self.generate_summary_text(&content);

        let report_id = Uuid::new_v4().to_string();
        let now = Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();

        sqlx::query(
            r#"INSERT INTO reports (id, type, title, period_start, period_end, content, summary, format, status, generated_at)
               VALUES (?, ?, ?, ?, ?, ?, ?, 'json', 'completed', ?)"#
        )
        .bind(&report_id)
        .bind(report_type)
        .bind(title)
        .bind(start)
        .bind(end)
        .bind(&content_json)
        .bind(&summary_text)
        .bind(&now)
        .execute(&self.db)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to save report: {}", e)))?;

        info!("Generated {} report: {}", report_type, report_id);

        Ok(Report {
            id: report_id,
            report_type: report_type.to_string(),
            title: title.to_string(),
            period_start: start.to_string(),
            period_end: end.to_string(),
            content: content_json,
            summary: Some(summary_text),
            format: "json".to_string(),
            status: "completed".to_string(),
            generated_at: now,
        })
    }

    async fn fetch_summary(&self, start: &str, end: &str) -> Result<ReportSummary, AppError> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM rule_executions WHERE detected_at >= ? AND detected_at <= ?"
        )
        .bind(start)
        .bind(end)
        .fetch_one(&self.db)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to count detections: {}", e)))?;
        let total_detections = row.0;

        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM rule_executions WHERE detected_at >= ? AND detected_at <= ? AND status IN ('open', 'acknowledged', 'investigating')"
        )
        .bind(start)
        .bind(end)
        .fetch_one(&self.db)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to count open: {}", e)))?;
        let open_detections = row.0;

        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM rule_executions WHERE detected_at >= ? AND detected_at <= ? AND status IN ('resolved', 'false_positive')"
        )
        .bind(start)
        .bind(end)
        .fetch_one(&self.db)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to count resolved: {}", e)))?;
        let resolved_detections = row.0;

        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM rule_executions re JOIN rules r ON re.rule_id = r.id WHERE re.detected_at >= ? AND re.detected_at <= ? AND r.severity = 'critical'"
        )
        .bind(start)
        .bind(end)
        .fetch_one(&self.db)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to count critical: {}", e)))?;
        let critical_count = row.0;

        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM rule_executions re JOIN rules r ON re.rule_id = r.id WHERE re.detected_at >= ? AND re.detected_at <= ? AND r.severity = 'high'"
        )
        .bind(start)
        .bind(end)
        .fetch_one(&self.db)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to count high: {}", e)))?;
        let high_count = row.0;

        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM rule_executions re JOIN rules r ON re.rule_id = r.id WHERE re.detected_at >= ? AND re.detected_at <= ? AND r.severity = 'medium'"
        )
        .bind(start)
        .bind(end)
        .fetch_one(&self.db)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to count medium: {}", e)))?;
        let medium_count = row.0;

        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM rule_executions re JOIN rules r ON re.rule_id = r.id WHERE re.detected_at >= ? AND re.detected_at <= ? AND r.severity = 'low'"
        )
        .bind(start)
        .bind(end)
        .fetch_one(&self.db)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to count low: {}", e)))?;
        let low_count = row.0;

        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(DISTINCT rule_id) FROM rule_executions WHERE detected_at >= ? AND detected_at <= ?"
        )
        .bind(start)
        .bind(end)
        .fetch_one(&self.db)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to count rules: {}", e)))?;
        let unique_rules_triggered = row.0;

        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(DISTINCT group_key) FROM rule_executions WHERE detected_at >= ? AND detected_at <= ? AND group_key IS NOT NULL"
        )
        .bind(start)
        .bind(end)
        .fetch_one(&self.db)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to count IPs: {}", e)))?;
        let unique_ips = row.0;

        Ok(ReportSummary {
            total_detections,
            open_detections,
            resolved_detections,
            critical_count,
            high_count,
            medium_count,
            low_count,
            unique_rules_triggered,
            unique_ips,
        })
    }

    async fn fetch_top_rules(&self, start: &str, end: &str, limit: i64) -> Result<Vec<RuleStat>, AppError> {
        let rows: Vec<(String, String, String, i64, i64)> = sqlx::query_as(
            r#"SELECT r.id, r.name, r.severity, COUNT(re.id) as detection_count, SUM(re.matched_count) as total_matched
               FROM rule_executions re
               JOIN rules r ON re.rule_id = r.id
               WHERE re.detected_at >= ? AND re.detected_at <= ?
               GROUP BY r.id
               ORDER BY detection_count DESC
               LIMIT ?"#
        )
        .bind(start)
        .bind(end)
        .bind(limit)
        .fetch_all(&self.db)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to fetch top rules: {}", e)))?;

        Ok(rows.into_iter().map(|(rule_id, rule_name, severity, detection_count, total_matched)| {
            RuleStat { rule_id, rule_name, severity, detection_count, total_matched }
        }).collect())
    }

    async fn fetch_top_ips(&self, start: &str, end: &str, limit: i64) -> Result<Vec<IpStat>, AppError> {
        let rows: Vec<(String, i64, i64)> = sqlx::query_as(
            r#"SELECT group_key as ip, COUNT(*) as detection_count, COUNT(DISTINCT rule_id) as rules_triggered
               FROM rule_executions
               WHERE detected_at >= ? AND detected_at <= ? AND group_key IS NOT NULL
               GROUP BY group_key
               ORDER BY detection_count DESC
               LIMIT ?"#
        )
        .bind(start)
        .bind(end)
        .bind(limit)
        .fetch_all(&self.db)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to fetch top IPs: {}", e)))?;

        Ok(rows.into_iter().map(|(ip, detection_count, rules_triggered)| {
            IpStat { ip, detection_count, rules_triggered }
        }).collect())
    }

    async fn fetch_severity_breakdown(&self, start: &str, end: &str) -> Result<SeverityBreakdown, AppError> {
        let rows: Vec<(String, i64)> = sqlx::query_as(
            r#"SELECT r.severity, COUNT(*) as cnt
               FROM rule_executions re
               JOIN rules r ON re.rule_id = r.id
               WHERE re.detected_at >= ? AND re.detected_at <= ?
               GROUP BY r.severity"#
        )
        .bind(start)
        .bind(end)
        .fetch_all(&self.db)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to fetch severity: {}", e)))?;

        let mut breakdown = SeverityBreakdown { critical: 0, high: 0, medium: 0, low: 0 };
        for (sev, cnt) in rows {
            match sev.as_str() {
                "critical" => breakdown.critical = cnt,
                "high" => breakdown.high = cnt,
                "medium" => breakdown.medium = cnt,
                "low" => breakdown.low = cnt,
                _ => {}
            }
        }
        Ok(breakdown)
    }

    async fn fetch_hourly_distribution(&self, start: &str, end: &str) -> Result<Vec<u32>, AppError> {
        let rows: Vec<(i32, i64)> = sqlx::query_as(
            r#"SELECT CAST(strftime('%H', detected_at) AS INTEGER) as hour, COUNT(*) as cnt
               FROM rule_executions
               WHERE detected_at >= ? AND detected_at <= ?
               GROUP BY hour
               ORDER BY hour"#
        )
        .bind(start)
        .bind(end)
        .fetch_all(&self.db)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to fetch hourly: {}", e)))?;

        let mut hourly = vec![0u32; 24];
        for (h, cnt) in rows {
            if h >= 0 && h < 24 {
                hourly[h as usize] = cnt as u32;
            }
        }
        Ok(hourly)
    }

    fn generate_summary_text(&self, content: &ReportContent) -> String {
        let s = &content.summary;
        format!(
            "기간: {} ~ {}\n총 탐지: {}건 (미해결: {}건, 해결됨: {}건)\n심각도: Critical {}건, High {}건, Medium {}건, Low {}건\n활성 룰: {}개, 고유 IP: {}개",
            content.period.start, content.period.end,
            s.total_detections, s.open_detections, s.resolved_detections,
            s.critical_count, s.high_count, s.medium_count, s.low_count,
            s.unique_rules_triggered, s.unique_ips
        )
    }

    pub async fn list_reports(&self, page: u32, size: u32) -> Result<(Vec<Report>, i64), AppError> {
        let offset = ((page - 1) * size) as i64;
        let limit = size as i64;

        let reports: Vec<Report> = sqlx::query_as::<_, Report>(
            "SELECT id, type as report_type, title, period_start, period_end, content, summary, format, status, generated_at FROM reports ORDER BY generated_at DESC LIMIT ? OFFSET ?"
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.db)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to list reports: {}", e)))?;

        let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM reports")
            .fetch_one(&self.db)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to count reports: {}", e)))?;

        Ok((reports, total.0))
    }

    pub async fn get_report(&self, id: &str) -> Result<Option<Report>, AppError> {
        let report = sqlx::query_as::<_, Report>(
            "SELECT id, type as report_type, title, period_start, period_end, content, summary, format, status, generated_at FROM reports WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&self.db)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to get report: {}", e)))?;

        Ok(report)
    }
}
