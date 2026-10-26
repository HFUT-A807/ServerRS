CREATE TABLE IF NOT EXISTS risk_mail_notifications (
    notification_id CHAR(36) PRIMARY KEY,
    user_id BIGINT UNSIGNED NOT NULL,
    conversation_id BIGINT UNSIGNED NULL,
    recipient VARCHAR(254) NOT NULL,
    display_name VARCHAR(255) NOT NULL,
    risk_level VARCHAR(16) NOT NULL,
    risk_audit_id BIGINT UNSIGNED NULL,
    is_test BOOLEAN NOT NULL DEFAULT FALSE,
    status VARCHAR(16) NOT NULL DEFAULT 'pending',
    attempts TINYINT UNSIGNED NOT NULL DEFAULT 0,
    successful_sends TINYINT UNSIGNED NOT NULL DEFAULT 0,
    next_send_at DATETIME(6) NOT NULL,
    last_attempt_at DATETIME(6) NULL,
    last_sent_at DATETIME(6) NULL,
    confirmed_at DATETIME(6) NULL,
    created_at DATETIME(6) NOT NULL,
    expires_at DATETIME(6) NOT NULL,
    active_key VARCHAR(350) GENERATED ALWAYS AS (
        IF(status = 'pending', CONCAT(user_id, ':', risk_level, ':', recipient), NULL)
    ) STORED,
    UNIQUE KEY uq_risk_mail_active (active_key),
    KEY idx_risk_mail_due (status, next_send_at),
    CONSTRAINT chk_risk_mail_attempts CHECK (attempts <= 3),
    CONSTRAINT chk_risk_mail_status CHECK (status IN ('pending', 'confirmed', 'cancelled', 'expired'))
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;
