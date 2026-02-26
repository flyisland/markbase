---
# [MKS v1.8 Template Definition]
_schema:
  description: >-
    工作相关人员档案模板。
    适用于客户侧联系人、合作伙伴、同事等所有与工作产生交集的人员。
    当对话流中出现人名、职务、所属公司、沟通记录等信息时匹配此模板。
    此模板设计为可扩展——随着关系深入，可逐步沉淀个人偏好、私人信息等，
    演变为通用人员档案。
  strict: false
  required: []

  filename:
    description: >-
      使用人名作为文件名。优先使用中文全名；
      若为外籍人士或惯用英文名，使用「FirstName_LastName」格式。
      示例：张伟、David_Chen。

  location: "entities/"

  properties:
    aliases:
      type: list
      description: "其他称呼、英文名或常用简称，如 ['张总', 'David']"
    relation:
      type: text
      enum: ["Customer", "Partner", "Colleague", "Vendor", "Other"]
      description: "与此人的关系类型"
    company:
      type: text
      format: link
      target: company
      description: "所属公司或组织，执行实体对齐"
    title:
      type: text
      description: "职务或头衔，如「采购总监」「CTO」"
    contact:
      type: text
      description: "主要联系方式，如「微信：david_chen / 邮箱：david@example.com」"
    status:
      type: text
      enum: ["Active", "Inactive", "Archived"]
      default: "Active"

type: person
template: person_work
tags: []
---

# {{ name }}

## 1. 基本印象
<!-- [Fill]:
     根据上下文，用 1-3 句话描述此人的核心角色、风格或值得注意的特点。
     若信息不足，此章节留空。
-->

## 2. 工作背景
<!-- [Update]: Overwrite
     当获取到更完整的职业背景信息时，更新此章节。
     包括：在公司中的职责范围、决策权限、所负责的业务线、汇报关系等。
     格式为自然段落，无需严格结构。
-->

## 3. 当前议题
<!-- [Update]: Overwrite
     记录此人当下最关心的事项、正在推动的项目、面临的压力或目标。
     每次更新时完全重写，只保留最新状态，并在段首标注更新日期。
     格式：`> 更新于 [[YYYY-MM-DD]]`
     用途：下次沟通前快速回顾，找到对话切入点。
-->

## 4. 关注点与偏好
<!-- [Update]: Accumulate
     当对话或互动中发现此人的关注点、偏好、顾虑、价值观或决策风格时，追加记录。
     不做去重或覆盖，保留全部历史以追踪观点演变。
     格式：`- <要点>（[[YYYY-MM-DD]]，来源：[[源文件链接]]）`
     追加前检查源文件路径是否已存在，存在则跳过（幂等）。
     可涵盖维度举例：
       - 技术偏好（如：偏好私有化部署，排斥 SaaS）
       - 决策风格（如：需要技术团队背书才推动采购）
       - 沟通偏好（如：不喜欢冗长 PPT，倾向直接数据）
       - 个人关切（如：对供应商稳定性要求高）
-->

## 5. 关键互动记录
<!-- [Update]: Append
     每次与此人产生有效互动（会议、电话、拜访、关键邮件）时，在末尾追加一条记录。
     格式：`- [[YYYY-MM-DD]] [<Type>] <简要描述> → [[源文件链接]]`
     Type 可选值：Meeting / Call / Visit / Email / WeChat / Other
     追加前检查源文件路径是否已存在，存在则跳过（幂等）。
-->

## 6. 关系网络
<!-- [Update]: Accumulate
     当发现此人与其他实体（人员、公司、项目）的关联时，追加记录。
     对齐失败时使用悬空引用 [?[实体名]] 占位。
     格式：`- 与 [[实体名]] 的关系：<描述>（[[YYYY-MM-DD]]）`
     追加前检查源文件路径是否已存在，存在则跳过（幂等）。
-->

## 7. 个人信息
