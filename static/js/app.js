// 应用主要JS功能
document.addEventListener('DOMContentLoaded', function() {
    // 初始化Toast
    const toastElList = [].slice.call(document.querySelectorAll('.toast'));
    const toastList = toastElList.map(function(toastEl) {
        return new bootstrap.Toast(toastEl);
    });

    // 显示成功消息
    function showSuccess(message) {
        showToast(message, 'success');
    }

    // 显示错误消息
    function showError(message) {
        showToast(message, 'error');
    }

    // 显示Toast消息
    function showToast(message, type = 'info') {
        const toastContainer = getOrCreateToastContainer();
        
        const toastHtml = `
            <div class="toast align-items-center text-bg-${type === 'error' ? 'danger' : type === 'success' ? 'success' : 'primary'} border-0" role="alert">
                <div class="d-flex">
                    <div class="toast-body">
                        <i class="bi bi-${type === 'error' ? 'exclamation-triangle' : type === 'success' ? 'check-circle' : 'info-circle'}"></i>
                        ${message}
                    </div>
                    <button type="button" class="btn-close btn-close-white me-2 m-auto" data-bs-dismiss="toast"></button>
                </div>
            </div>
        `;
        
        toastContainer.insertAdjacentHTML('beforeend', toastHtml);
        const newToast = toastContainer.lastElementChild;
        const toast = new bootstrap.Toast(newToast);
        toast.show();
        
        // 自动删除DOM元素
        newToast.addEventListener('hidden.bs.toast', function() {
            newToast.remove();
        });
    }

    // 获取或创建Toast容器
    function getOrCreateToastContainer() {
        let container = document.getElementById('toast-container');
        if (!container) {
            container = document.createElement('div');
            container.id = 'toast-container';
            container.className = 'toast-container position-fixed bottom-0 end-0 p-3';
            container.style.zIndex = '1055';
            document.body.appendChild(container);
        }
        return container;
    }

    // 表单验证增强
    const forms = document.querySelectorAll('.needs-validation');
    forms.forEach(function(form) {
        form.addEventListener('submit', function(event) {
            if (!form.checkValidity()) {
                event.preventDefault();
                event.stopPropagation();
            }
            form.classList.add('was-validated');
        });
    });

    // 确认删除对话框
    document.addEventListener('click', function(e) {
        if (e.target.classList.contains('confirm-delete')) {
            e.preventDefault();
            const message = e.target.dataset.message || '确定要删除这条记录吗？';
            
            if (confirm(message)) {
                if (e.target.tagName === 'A') {
                    window.location.href = e.target.href;
                } else if (e.target.tagName === 'BUTTON' && e.target.form) {
                    e.target.form.submit();
                }
            }
        }
    });

    // 金额输入格式化
    const amountInputs = document.querySelectorAll('input[type="number"][step="0.01"]');
    amountInputs.forEach(function(input) {
        input.addEventListener('blur', function() {
            if (this.value) {
                this.value = parseFloat(this.value).toFixed(2);
            }
        });
    });

    // 自动完成功能（用于标签输入）
    const tagInputs = document.querySelectorAll('.tag-input');
    tagInputs.forEach(function(input) {
        input.addEventListener('keydown', function(e) {
            if (e.key === 'Enter' && !e.shiftKey) {
                e.preventDefault();
                // 这里可以添加标签选择逻辑
            }
        });
    });

    // 侧边栏切换（移动端）
    const sidebarToggle = document.getElementById('sidebarToggle');
    if (sidebarToggle) {
        sidebarToggle.addEventListener('click', function() {
            document.body.classList.toggle('sidebar-open');
        });
    }

    // 搜索功能
    const searchInputs = document.querySelectorAll('.search-input');
    searchInputs.forEach(function(input) {
        let timeout;
        input.addEventListener('input', function() {
            clearTimeout(timeout);
            timeout = setTimeout(() => {
                // 这里可以添加实时搜索逻辑
                console.log('搜索:', this.value);
            }, 300);
        });
    });

    // 日期选择器增强
    const dateInputs = document.querySelectorAll('input[type="date"]');
    dateInputs.forEach(function(input) {
        if (!input.value) {
            input.value = new Date().toISOString().split('T')[0];
        }
    });

    // 工具提示初始化
    const tooltipTriggerList = [].slice.call(document.querySelectorAll('[data-bs-toggle="tooltip"]'));
    const tooltipList = tooltipTriggerList.map(function(tooltipTriggerEl) {
        return new bootstrap.Tooltip(tooltipTriggerEl);
    });

    // 弹出框初始化
    const popoverTriggerList = [].slice.call(document.querySelectorAll('[data-bs-toggle="popover"]'));
    const popoverList = popoverTriggerList.map(function(popoverTriggerEl) {
        return new bootstrap.Popover(popoverTriggerEl);
    });

    // URL参数处理
    const urlParams = new URLSearchParams(window.location.search);
    const successMessage = urlParams.get('success');
    const errorMessage = urlParams.get('error');
    
    if (successMessage) {
        showSuccess(decodeURIComponent(successMessage));
        // 清除URL参数
        const url = new URL(window.location);
        url.searchParams.delete('success');
        window.history.replaceState({}, document.title, url);
    }
    
    if (errorMessage) {
        showError(decodeURIComponent(errorMessage));
        // 清除URL参数
        const url = new URL(window.location);
        url.searchParams.delete('error');
        window.history.replaceState({}, document.title, url);
    }

    // 全局错误处理
    window.addEventListener('error', function(e) {
        console.error('JavaScript Error:', e.error);
        showError('发生了一个错误，请刷新页面重试');
    });

    // 全局函数暴露
    window.AccountingSystem = {
        showSuccess,
        showError,
        showToast
    };
});

// 工具函数
function formatCurrency(amount) {
    return new Intl.NumberFormat('zh-CN', {
        style: 'currency',
        currency: 'CNY'
    }).format(amount);
}

function formatDate(dateString) {
    return new Date(dateString).toLocaleDateString('zh-CN');
}

function debounce(func, wait) {
    let timeout;
    return function executedFunction(...args) {
        const later = () => {
            clearTimeout(timeout);
            func(...args);
        };
        clearTimeout(timeout);
        timeout = setTimeout(later, wait);
    };
}