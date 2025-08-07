# Rust Web 记账系统

一个基于 Rust 和 Axum 框架开发的现代化 Web 记账系统，支持多账本管理、分类统计、月度报表等功能。

## 功能特性

- 🔐 **用户认证系统** - 邮箱注册、登录、验证
- 📚 **多账本管理** - 创建和管理多个记账本
- 🏷️ **分类管理** - 自定义收入和支出分类
- 💰 **记账功能** - 快速记录收入和支出
- 📊 **统计报表** - 月度统计和分类分析
- 📱 **响应式设计** - 支持桌面端和移动端
- ⚡ **高性能** - 基于 Rust 异步框架

## 技术栈

- **后端框架**: Axum (异步 Web 框架)
- **数据库**: MySQL
- **ORM**: SQLx (类型安全的数据库操作)
- **模板引擎**: Askama (编译时模板)
- **认证**: JWT + Cookie Session
- **邮件服务**: Lettre
- **密码加密**: Argon2
- **前端**: Bootstrap 5 + 原生 JavaScript

## 快速开始

### 环境要求

- Rust 1.70+
- MySQL 8.0+
- Node.js (可选，用于前端开发)

### 1. 克隆项目

```bash
git clone <repository-url>
cd accounting-system
```

### 2. 数据库设置

1. 创建 MySQL 数据库：
```sql
CREATE DATABASE accounting_system CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;
```

2. 运行数据库脚本：
```bash
mysql -u your_username -p accounting_system < database_schema.sql
```

### 3. 环境配置

1. 复制环境变量文件：
```bash
cp .env.example .env
```

2. 编辑 `.env` 文件，配置数据库连接和邮件服务：
```env
DATABASE_URL=mysql://username:password@localhost:3306/accounting_system
SMTP_HOST=smtp.gmail.com
SMTP_USERNAME=your-email@gmail.com
SMTP_PASSWORD=your-app-password
FROM_EMAIL=your-email@gmail.com
```

### 4. 运行应用

```bash
# 安装依赖并运行
cargo run

# 或者编译后运行
cargo build --release
./target/release/accounting-system
```

应用将在 `http://localhost:3000` 启动。

## 项目结构

```
accounting-system/
├── src/
│   ├── main.rs                 # 应用入口
│   ├── lib.rs                  # 库入口
│   ├── config.rs               # 配置管理
│   ├── models/                 # 数据模型
│   ├── handlers/               # 路由处理器
│   ├── services/               # 业务逻辑
│   ├── middleware/             # 中间件
│   ├── database/               # 数据库连接
│   └── utils/                  # 工具函数
├── templates/                  # Askama 模板
├── static/                     # 静态资源
├── database_schema.sql         # 数据库结构
├── .env.example               # 环境变量示例
└── Cargo.toml                 # 项目配置
```

## API 路由

### 认证相关
- `GET /` - 首页
- `GET /auth/login` - 登录页面
- `POST /auth/login` - 用户登录
- `GET /auth/register` - 注册页面
- `POST /auth/register` - 用户注册
- `GET /auth/verify/:token` - 邮箱验证
- `POST /auth/logout` - 用户登出

### 主要功能
- `GET /dashboard` - 用户仪表板
- `GET /account-books` - 账本列表
- `POST /account-books` - 创建账本
- `GET /account-books/:id/categories` - 分类管理
- `GET /account-books/:id/transactions` - 交易记录
- `POST /transactions` - 创建交易记录

## 开发

### 运行开发服务器

```bash
# 启用日志
RUST_LOG=debug cargo run
```

### 数据库迁移

如果需要修改数据库结构，请更新 `database_schema.sql` 文件。

### 前端开发

静态资源位于 `static/` 目录：
- `static/css/style.css` - 自定义样式
- `static/js/app.js` - JavaScript 功能
- `static/images/` - 图片资源

## 部署

### 1. 编译生产版本

```bash
cargo build --release
```

### 2. 环境变量

确保生产环境设置了正确的环境变量：

```env
DATABASE_URL=mysql://user:pass@localhost:3306/accounting_system
JWT_SECRET=your-production-jwt-secret
SESSION_SECRET=your-production-session-secret
SMTP_HOST=your-smtp-server
```

### 3. 反向代理

推荐使用 Nginx 作为反向代理：

```nginx
server {
    listen 80;
    server_name your-domain.com;
    
    location / {
        proxy_pass http://127.0.0.1:3000;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }
    
    location /static/ {
        alias /path/to/accounting-system/static/;
        expires 1y;
    }
}
```

## 贡献

1. Fork 项目
2. 创建功能分支 (`git checkout -b feature/amazing-feature`)
3. 提交更改 (`git commit -m 'Add some amazing feature'`)
4. 推送到分支 (`git push origin feature/amazing-feature`)
5. 创建 Pull Request

## 许可证

本项目采用 MIT 许可证 - 查看 [LICENSE](LICENSE) 文件了解详情。

## 联系方式

如有问题或建议，请创建 [Issue](https://github.com/your-username/accounting-system/issues)。