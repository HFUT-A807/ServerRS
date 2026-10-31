-- Standard GDS-15, simplified Chinese wording and screening bands from NARI.
-- https://web.stanford.edu/~yesavage/Australian%20Chinese.pdf (pages 1-2)
-- Items 1,5,7,11,13: Yes=0, No=1; all others: Yes=1, No=0.
-- Insert only: never overwrite an existing scale or historical assessment.
SET NAMES utf8mb4 COLLATE utf8mb4_unicode_ci;
START TRANSACTION;
INSERT INTO depression_scales
    (scale_id, scale_name, scale_description, min_score, max_score, severity_ranges, questions)
SELECT 4, 'GDS-15',
 '老年抑郁量表15项简版（GDS-15），采用标准版条目及澳大利亚国家老龄研究所（NARI）提供的简体中文译文。请根据过去一周的感受回答是或否。每题0或1分，总分0–15分；第1、5、7、11、13题回答否计1分，其余题回答是计1分。采用该中文版本的筛查参考区间：0–4分未提示抑郁症状，5–9分提示轻度症状，10–15分提示中度症状。分界值随版本和人群而异，结果仅用于老年人抑郁症状筛查，不能代替临床诊断；5分及以上建议接受专业评估。来源：https://web.stanford.edu/~yesavage/Australian%20Chinese.pdf。',
 0, 15,
 JSON_ARRAY(
   JSON_OBJECT('min',0,'max',4,'level','未提示抑郁症状','label','未提示抑郁症状','guidance','筛查结果未提示抑郁症状；如仍有持续困扰，可咨询专业人员'),
   JSON_OBJECT('min',5,'max',9,'level','提示轻度抑郁症状','label','提示轻度抑郁症状','guidance','建议接受专业评估，结合症状及日常功能判断；本结果不是诊断'),
   JSON_OBJECT('min',10,'max',15,'level','提示中度抑郁症状','label','提示中度抑郁症状','guidance','建议尽快接受专业评估；本结果不是诊断')
 ),
 JSON_ARRAY(
   JSON_OBJECT('id',1,'text','你对自己的生活基本上满意吗？','period','过去一周','reverse_scored',true,'options',JSON_ARRAY(JSON_OBJECT('score',0,'label','是'),JSON_OBJECT('score',1,'label','否'))),
   JSON_OBJECT('id',2,'text','你是否已放弃了自己的很多活动和兴趣？','period','过去一周','reverse_scored',false,'options',JSON_ARRAY(JSON_OBJECT('score',1,'label','是'),JSON_OBJECT('score',0,'label','否'))),
   JSON_OBJECT('id',3,'text','你是否觉得生活空虚？','period','过去一周','reverse_scored',false,'options',JSON_ARRAY(JSON_OBJECT('score',1,'label','是'),JSON_OBJECT('score',0,'label','否'))),
   JSON_OBJECT('id',4,'text','你是否常常感到无聊？','period','过去一周','reverse_scored',false,'options',JSON_ARRAY(JSON_OBJECT('score',1,'label','是'),JSON_OBJECT('score',0,'label','否'))),
   JSON_OBJECT('id',5,'text','你是否常常感到精神不错，精神还可以？','period','过去一周','reverse_scored',true,'options',JSON_ARRAY(JSON_OBJECT('score',0,'label','是'),JSON_OBJECT('score',1,'label','否'))),
   JSON_OBJECT('id',6,'text','你是否害怕会有不好的事情发生在你身上呢？','period','过去一周','reverse_scored',false,'options',JSON_ARRAY(JSON_OBJECT('score',1,'label','是'),JSON_OBJECT('score',0,'label','否'))),
   JSON_OBJECT('id',7,'text','你大部分时间心情还可以吗？','period','过去一周','reverse_scored',true,'options',JSON_ARRAY(JSON_OBJECT('score',0,'label','是'),JSON_OBJECT('score',1,'label','否'))),
   JSON_OBJECT('id',8,'text','你是否经常觉得无助？','period','过去一周','reverse_scored',false,'options',JSON_ARRAY(JSON_OBJECT('score',1,'label','是'),JSON_OBJECT('score',0,'label','否'))),
   JSON_OBJECT('id',9,'text','你是否宁愿留在家里，而不外出并做些新的事呢？','period','过去一周','reverse_scored',false,'options',JSON_ARRAY(JSON_OBJECT('score',1,'label','是'),JSON_OBJECT('score',0,'label','否'))),
   JSON_OBJECT('id',10,'text','你是否觉得你的记忆力比多数人差？','period','过去一周','reverse_scored',false,'options',JSON_ARRAY(JSON_OBJECT('score',1,'label','是'),JSON_OBJECT('score',0,'label','否'))),
   JSON_OBJECT('id',11,'text','你是否觉得活着真好，活着还不错？','period','过去一周','reverse_scored',true,'options',JSON_ARRAY(JSON_OBJECT('score',0,'label','是'),JSON_OBJECT('score',1,'label','否'))),
   JSON_OBJECT('id',12,'text','你是否觉得自己现在很没用呢？','period','过去一周','reverse_scored',false,'options',JSON_ARRAY(JSON_OBJECT('score',1,'label','是'),JSON_OBJECT('score',0,'label','否'))),
   JSON_OBJECT('id',13,'text','你是否感到精力充足，精力足够应付日常的生活？','period','过去一周','reverse_scored',true,'options',JSON_ARRAY(JSON_OBJECT('score',0,'label','是'),JSON_OBJECT('score',1,'label','否'))),
   JSON_OBJECT('id',14,'text','你是否觉得自己的处境没有希望？','period','过去一周','reverse_scored',false,'options',JSON_ARRAY(JSON_OBJECT('score',1,'label','是'),JSON_OBJECT('score',0,'label','否'))),
   JSON_OBJECT('id',15,'text','你是否觉得多数人都比你活得好吗？','period','过去一周','reverse_scored',false,'options',JSON_ARRAY(JSON_OBJECT('score',1,'label','是'),JSON_OBJECT('score',0,'label','否')))
 )
WHERE NOT EXISTS (SELECT 1 FROM depression_scales WHERE scale_name = 'GDS-15');
COMMIT;
