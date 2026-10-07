CREATE TABLE wellbeing_recommendations
(
    id            BIGINT UNSIGNED PRIMARY KEY AUTO_INCREMENT,
    user_id       BIGINT UNSIGNED NOT NULL,
    assessment_id BIGINT UNSIGNED NOT NULL,
    activity      VARCHAR(64) NOT NULL,
    title         VARCHAR(100) NOT NULL,
    description   TEXT NOT NULL,
    status        VARCHAR(16) NOT NULL DEFAULT 'pending',
    available_at  DATETIME(6) NOT NULL,
    created_at    DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    updated_at    DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
    CONSTRAINT chk_wellbeing_status CHECK (status IN ('pending', 'completed', 'dismissed')),
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (assessment_id) REFERENCES depression_assessments(assessment_id) ON DELETE CASCADE,
    UNIQUE KEY uq_wellbeing_assessment_activity (assessment_id, activity),
    INDEX idx_wellbeing_due (user_id, status, available_at, id)
) ENGINE = InnoDB DEFAULT CHARSET = utf8mb4 COLLATE = utf8mb4_unicode_ci
  COMMENT = '健康促进建议；接口返回与低频轮询共用持久化记录';
