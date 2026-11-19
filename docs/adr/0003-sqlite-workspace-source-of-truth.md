---
status: accepted
---

# 单一本地工作区以 SQLite 为权威数据源

Kanon 以 SQLite 保存单一本地工作区，不以可外部编辑的 Markdown 文件作为主库。创建任务并在文档中插入关联任务卡片等跨对象操作必须原子提交，不能留下只写入一半的状态。Markdown 用于内容交换；完整备份必须能恢复对象及其关联，不能用 Markdown 导出替代。
