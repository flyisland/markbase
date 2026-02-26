---
# [MKS v1.7 Template Definition]
_schema:
  description: >-
    标准客户档案模版。
    用于建立新客户的基本信息库，并随着商机推进自动沉淀技术栈、关键人和活动记录。
  strict: false
  required: ["name", "industry", "owner"]

  properties:
    name:
      type: text
      description: "客户简称"
    industry:
      type: text
      enum: ["IoT", "Automotive", "Finance", "Gaming"]
      description: "客户所属行业"
    owner:
      type: text
      format: link
      target: person
      description: "内部销售负责人"
    status:
      type: text
      enum: ["Lead", "POC", "Customer"]
      default: "Lead"

type: company
template: company_customer.md
tags: ["customer"]
---

# {{ name }} 客户档案

## 1. 企业概况
<!-- [Fill]:
     根据对话上下文，用 2-3 句话概括客户的主营业务、规模和核心产品。
     若上下文信息不足，此章节留空。
-->

## 2. 组织架构与关键人
<!-- [Update]: Accumulate
     当工作日记或会议纪要中出现客户侧人员信息时，提取并追加到此章节。
     每个人员须完成实体对齐：
       - 对齐成功：使用双链格式 [[人名]]
       - 对齐不确定：使用悬空引用 [?[人名]]，待后续确认
     格式：`- [[人名]] — <职务/角色>（首次出现：[[YYYY-MM-DD]]）`
     若同一人员的职务或角色有变动，在原条目后追加备注，保留历史：
     `- [[张伟]] — 采购总监（首次出现：[[2026-01-10]]）→ 升任 VP（[[2026-06-01]]）`
     追加前检查源文件路径是否已存在，存在则跳过（幂等）。
-->

## 3. 技术栈画像
<!-- [Update]: Accumulate
     当会议纪要或对话中出现新的技术信息时，在对应分类下追加一条记录。
     格式：`- <Category>: <Technology>（<Status>）— [[YYYY-MM-DD]]`
     Status 可选值：Evaluating / Planned / In Use / Deprecated
     即使新技术替代了旧技术，也保留旧条目，以记录演进轨迹。
     示例：
       - AI Coding: GitLab CE（Deprecated）— [[2025-06-01]]
       - AI Coding: GitLab Duo（Planned）— [[2026-02-26]]
     追加前检查源文件路径是否已存在，存在则跳过（幂等）。
-->
- **CI/CD**:
- **Cloud**:
- **Languages**:

## 4. 关键活动记录
<!-- [Update]: Append
     每次有新的客户互动（拜访、会议、电话）时，在末尾追加一条记录。
     格式：`- [[YYYY-MM-DD]] [<Type>] <简要描述> → [[源文件链接]]`
     Type 可选值：Visit / Call / Demo / Email
     追加前检查源文件路径是否已存在，存在则跳过（幂等）。
-->