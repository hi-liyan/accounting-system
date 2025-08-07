-- 记账系统数据库创建脚本
-- 创建数据库
CREATE DATABASE IF NOT EXISTS accounting_system CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;
USE accounting_system;

-- 用户表
CREATE TABLE users (
    id BIGINT PRIMARY KEY AUTO_INCREMENT COMMENT '用户ID',
    email VARCHAR(255) UNIQUE NOT NULL COMMENT '邮箱地址',
    username VARCHAR(100) NOT NULL COMMENT '用户名',
    password_hash VARCHAR(255) NOT NULL COMMENT '密码哈希',
    is_verified BOOLEAN DEFAULT FALSE COMMENT '邮箱是否已验证',
    verification_token VARCHAR(255) COMMENT '邮箱验证令牌',
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP COMMENT '创建时间',
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP COMMENT '更新时间',
    INDEX idx_email (email),
    INDEX idx_verification_token (verification_token)
) ENGINE=InnoDB COMMENT='用户表';

-- 账本表
CREATE TABLE account_books (
    id BIGINT PRIMARY KEY AUTO_INCREMENT COMMENT '账本ID',
    user_id BIGINT NOT NULL COMMENT '用户ID',
    name VARCHAR(255) NOT NULL COMMENT '账本名称',
    description TEXT COMMENT '账本描述',
    currency VARCHAR(3) DEFAULT 'CNY' COMMENT '货币类型',
    cycle_start_day INT DEFAULT 1 COMMENT '月度周期起始日(1-31)',
    is_active BOOLEAN DEFAULT TRUE COMMENT '是否激活',
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP COMMENT '创建时间',
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP COMMENT '更新时间',
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    INDEX idx_user_id (user_id),
    INDEX idx_name (name)
) ENGINE=InnoDB COMMENT='账本表';

-- 分类表
CREATE TABLE categories (
    id BIGINT PRIMARY KEY AUTO_INCREMENT COMMENT '分类ID',
    account_book_id BIGINT NOT NULL COMMENT '账本ID',
    name VARCHAR(100) NOT NULL COMMENT '分类名称',
    type ENUM('income', 'expense') NOT NULL COMMENT '分类类型：收入/支出',
    icon VARCHAR(50) DEFAULT 'default' COMMENT '图标名称',
    color VARCHAR(7) DEFAULT '#007bff' COMMENT '颜色代码',
    sort_order INT DEFAULT 0 COMMENT '排序顺序',
    is_active BOOLEAN DEFAULT TRUE COMMENT '是否激活',
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP COMMENT '创建时间',
    FOREIGN KEY (account_book_id) REFERENCES account_books(id) ON DELETE CASCADE,
    INDEX idx_account_book_id (account_book_id),
    INDEX idx_type (type),
    UNIQUE KEY uk_account_book_name (account_book_id, name)
) ENGINE=InnoDB COMMENT='分类表';

-- 交易记录表
CREATE TABLE transactions (
    id BIGINT PRIMARY KEY AUTO_INCREMENT COMMENT '交易ID',
    account_book_id BIGINT NOT NULL COMMENT '账本ID',
    category_id BIGINT NOT NULL COMMENT '分类ID',
    amount DECIMAL(15,2) NOT NULL COMMENT '金额',
    type ENUM('income', 'expense') NOT NULL COMMENT '交易类型：收入/支出',
    description TEXT COMMENT '交易描述',
    transaction_date DATE NOT NULL COMMENT '交易日期',
    tags VARCHAR(500) COMMENT '标签（逗号分隔）',
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP COMMENT '创建时间',
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP COMMENT '更新时间',
    FOREIGN KEY (account_book_id) REFERENCES account_books(id) ON DELETE CASCADE,
    FOREIGN KEY (category_id) REFERENCES categories(id),
    INDEX idx_account_book_id (account_book_id),
    INDEX idx_category_id (category_id),
    INDEX idx_transaction_date (transaction_date),
    INDEX idx_type (type),
    INDEX idx_created_at (created_at)
) ENGINE=InnoDB COMMENT='交易记录表';

-- 会话表（用于用户登录状态管理）
CREATE TABLE sessions (
    id VARCHAR(128) PRIMARY KEY COMMENT '会话ID',
    user_id BIGINT NOT NULL COMMENT '用户ID',
    expires_at TIMESTAMP NOT NULL COMMENT '过期时间',
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP COMMENT '创建时间',
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    INDEX idx_user_id (user_id),
    INDEX idx_expires_at (expires_at)
) ENGINE=InnoDB COMMENT='用户会话表';

-- 插入默认分类数据（支出类别）
INSERT INTO categories (account_book_id, name, type, icon, color, sort_order) VALUES 
-- 注意：这里的account_book_id需要在创建账本后动态插入，这里仅作为示例
-- (1, '餐饮美食', 'expense', 'restaurant', '#ff6b6b', 1),
-- (1, '交通出行', 'expense', 'car', '#4ecdc4', 2),
-- (1, '购物消费', 'expense', 'shopping', '#45b7d1', 3),
-- (1, '娱乐休闲', 'expense', 'game', '#f7b731', 4),
-- (1, '医疗健康', 'expense', 'medical', '#5f27cd', 5),
-- (1, '学习教育', 'expense', 'book', '#00d2d3', 6),
-- (1, '住房租金', 'expense', 'home', '#ff9ff3', 7),
-- (1, '水电燃气', 'expense', 'utility', '#ff3838', 8),
-- (1, '通讯网费', 'expense', 'phone', '#17c0eb', 9),
-- (1, '其他支出', 'expense', 'other', '#a4b0be', 10);

-- 插入默认分类数据（收入类别）
-- INSERT INTO categories (account_book_id, name, type, icon, color, sort_order) VALUES 
-- (1, '工资收入', 'income', 'salary', '#26de81', 1),
-- (1, '兼职收入', 'income', 'work', '#4b7bec', 2),
-- (1, '投资理财', 'income', 'investment', '#fd79a8', 3),
-- (1, '礼金红包', 'income', 'gift', '#fdcb6e', 4),
-- (1, '其他收入', 'income', 'other', '#6c5ce7', 5);

-- 创建数据库视图 - 月度统计
CREATE VIEW monthly_stats AS
SELECT 
    ab.id as account_book_id,
    ab.name as account_book_name,
    ab.user_id,
    YEAR(t.transaction_date) as year,
    MONTH(t.transaction_date) as month,
    t.type,
    SUM(t.amount) as total_amount,
    COUNT(*) as transaction_count
FROM account_books ab
JOIN transactions t ON ab.id = t.account_book_id
GROUP BY ab.id, ab.name, ab.user_id, YEAR(t.transaction_date), MONTH(t.transaction_date), t.type;

-- 创建数据库视图 - 分类统计
CREATE VIEW category_stats AS
SELECT 
    c.id as category_id,
    c.name as category_name,
    c.type,
    c.account_book_id,
    ab.user_id,
    COUNT(t.id) as transaction_count,
    COALESCE(SUM(t.amount), 0) as total_amount,
    AVG(t.amount) as avg_amount
FROM categories c
LEFT JOIN transactions t ON c.id = t.category_id
JOIN account_books ab ON c.account_book_id = ab.id
GROUP BY c.id, c.name, c.type, c.account_book_id, ab.user_id;

-- 清理过期会话的存储过程
DELIMITER //
CREATE PROCEDURE CleanExpiredSessions()
BEGIN
    DELETE FROM sessions WHERE expires_at < NOW();
END //
DELIMITER ;

-- 创建定时任务清理过期会话（需要MySQL事件调度器开启）
-- SET GLOBAL event_scheduler = ON;
-- CREATE EVENT IF NOT EXISTS clean_expired_sessions
-- ON SCHEDULE EVERY 1 HOUR
-- DO CALL CleanExpiredSessions();