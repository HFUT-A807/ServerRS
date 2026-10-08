-- Clarify the SDS rounding convention without changing scale IDs, questions,
-- option scores, score bounds, severity ranges or historical assessments.
START TRANSACTION;
UPDATE depression_scales
SET scale_description = REPLACE(
    scale_description,
    '原始分×1.25后取整数',
    '原始分×1.25后四舍五入取整数'
)
WHERE scale_name = 'SDS'
  AND scale_description LIKE '%原始分×1.25后取整数%';
COMMIT;
