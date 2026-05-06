# PRD - 貿易書類&受注FAX AIOCR System
## Product Requirements Document

> **Ngày tạo**: 2026-04-07
> **Dự án**: AIOCR for Trade Documents & Order FAX (貿易書類&受注FAX AIOCR)
> **Phiên bản**: 1.0
> **Trạng thái**: Draft

---

## Cấu trúc tài liệu

| # | File | Nội dung |
|---|------|----------|
| 01 | [Tổng quan hệ thống](01_SYSTEM_OVERVIEW.md) | Mục tiêu, phạm vi, stakeholder |
| 02 | [Kiến trúc hệ thống](02_ARCHITECTURE.md) | Architecture diagrams, tech stack, data flow |
| 03 | [User Flows](03_USER_FLOWS.md) | Luồng người dùng, workflow, state diagrams |
| 04 | [Wireframes](04_WIREFRAMES.md) | Thiết kế màn hình chi tiết (ASCII) |
| 05 | [Data Model](05_DATA_MODEL.md) | Database schema, entity relationships |
| 06 | [Functional Spec](06_FUNCTIONAL_SPEC.md) | Chi tiết tính năng, API, business rules |
| 07 | [Technical Spec](07_TECHNICAL_SPEC.md) | Tech stack, environment, estimate, team |

---

## Thông tin dự án

- **Khách hàng**: 不二貿易 (Fuji Boeki)
- **Mục tiêu**: DX (Digital Transformation) cho nghiệp vụ thương mại & nhận đơn hàng
- **Phạm vi**: 10 công ty đối tác, 30 form patterns, ~10 PDFs/case


---

# 01. Tổng quan hệ thống (System Overview)

## 1.1 Mục tiêu dự án

Xây dựng hệ thống AIOCR (AI-based OCR) để số hóa (DX) nghiệp vụ xử lý tài liệu thương mại quốc tế:
- **Invoice** (3-5 trang)
- **Packing List - PL** (~2 trang)
- **Bill of Lading - B/L** (~2 trang)

### Quy trình hiện tại (AS-IS)

```
+----------+    +--------------+    +--------------+    +--------------+
|  発注     |--->|  書類送付     |--->|  目視確認     |--->|  手入力       |
| (Đặt hàng)|    | (Gửi tài liệu)|    | (Kiểm tra mắt)|    | (Nhập tay)    |
+----------+    +--------------+    +--------------+    +--------------+
                                                            |
                                                            v
                                                     +--------------+
                                                     |  TOSS 反映    |
                                                     | (Nhập TOSS)   |
                                                     +--------------+
     [X] Mất nhiều thời gian
     [X] Dễ sai sót do nhập tay
     [X] Khó kiểm tra tính nhất quán giữa các tài liệu
```

### Quy trình mới (TO-BE)

```
+----------+    +--------------+    +--------------------------------------+
|  発注     |--->|  PDF Upload   |--->|  AIOCR Xử lý tự động                 |
| (Đặt hàng)|    | (Drag & Drop) |    |  - Trích xuất dữ liệu Invoice/PL/B/L |
+----------+    +--------------+    |  - Kiểm tra tính nhất quán           |
                                    |  - Tô màu theo confidence level       |
                                    +--------------+-----------------------+
                                                    |
                                                    v
                                    +--------------------------------------+
                                    |  Người dùng xác nhận & chỉnh sửa     |
                                    |  - Split view: Ảnh vs Dữ liệu        |
                                    |  - Free input cho items đặc biệt     |
                                    |  - Sắp xếp thứ tự output (drag&drop) |
                                    +--------------+-----------------------+
                                                    |
                                                    v
                                    +--------------------------------------+
                                    |  CSV Download                        |
                                    |  - Xuất CSV theo format TOSS         |
                                    |  - Phản ánh vào hệ thống TOSS        |
                                    +--------------------------------------+
     [OK] Giảm 70% thời gian xử lý
     [OK] Giảm thiểu sai sót
     [OK] Kiểm tra nhất quán tự động giữa Invoice/PL/B/L
```

## 1.2 Phạm vi dự án

### Trong phạm vi (In Scope)

```
+---------------------------------------------------------+
|                    PHẠM VI AIOCR                         |
|                                                          |
|  +-------------+  +-------------+  +-------------+       |
|  |  Invoice    |  | Packing List|  |    B/L       |       |
|  |  (3-5 pg)   |  |  (~2 pg)    |  |  (~2 pg)     |       |
|  +------+------+  +------+------+  +------+------+       |
|         |               |               |                |
|         v               v               v                |
|  +--------------------------------------------------+   |
|  |         AI OCR Engine (Printed text)              |   |
|  |         - Trích xuất dữ liệu có cấu trúc          |   |
|  |         - Confidence scoring                      |   |
|  |         - Cross-document validation               |   |
|  +----------------------+----------------------------+   |
|                         |                               |
|         +---------------+---------------+               |
|         v               v               v               |
|  +-------------+  +-------------+  +-------------+       |
|  |  Case       |  |  Partner    |  |  CSV        |       |
|  |  Management |  |  Master     |  |  Export     |       |
|  +-------------+  +-------------+  +-------------+       |
|                                                          |
|  +-------------+  +-------------+                         |
|  |  Free Input |  |  TOSS       |                         |
|  |  (30 items) |  |  Integration|                         |
|  +-------------+  +-------------+                         |
+---------------------------------------------------------+
```

### Ngoài phạm vi (Out of Scope)

- AI OCR cho chữ viết tay (手書きOCR) -- phát triển sau
- Tích hợp trực tiếp API với TOSS (chỉ xuất CSV)
- Xử lý tài liệu không phải Invoice/PL/B/L

### Thông số mục tiêu

| Thông số | Giá trị |
|----------|---------|
| Số công ty đối tác | 10 công ty (chiếm 70% khối lượng thương mại) |
| Số form patterns | 30 patterns |
| Số PDF trung bình/case | ~10 PDFs |
| Loại tài liệu | Invoice, Packing List, B/L |
| Output | CSV (cho TOSS system) |

## 1.3 Stakeholders

```
+-----------------------------------------------------+
|                   STAKEHOLDERS                       |
|                                                      |
|  +--------------+     +--------------+               |
|  | 不二貿易      |     | Development  |               |
|  | (Fuji Boeki) |---->| Team         |               |
|  | - Product    |<----| - Team Lead  |               |
|  |   Owner      |     | - Developer  |               |
|  +--------------+     | - UI Designer|               |
|          |            | - Tester     |               |
|          v            +--------------+               |
|  +--------------+                                     |
|  | Trade Ops    |     +--------------+               |
|  | Team         |---->| IT Infra     |               |
|  | - End Users  |     | - Server     |               |
|  | (10 công ty) |     | - Database   |               |
|  +--------------+     +--------------+               |
+-----------------------------------------------------+
```

## 1.4 Terminology

| Thuật ngữ | Tiếng Nhật | Ý nghĩa |
|-----------|-----------|---------|
| Invoice | インボイス | Hóa đơn thương mại |
| Packing List | パッキングリスト (PL) | Danh sách đóng gói |
| Bill of Lading | B/L | Vận đơn đường biển |
| Case | ケース | Đơn vị quản lý theo 発注番号 (số đặt hàng) |
| TOSS | TOSS | Hệ thống quản lý nội bộ |
| Confidence | 信頼度 | Độ tin cậy của AI OCR |
| Free Input | フリー入力 | Nhập tự do cho items không OCR được |
| Partner Master | 取引先マスタ | Master data đối tác |


---

# 02. Kiến trúc hệ thống (System Architecture)

## 2.1 System Architecture Overview

```
+---------------------------------------------------------------------+
|                        AIOCR SYSTEM ARCHITECTURE                    |
|                                                                     |
|  +-------------------------------------------------------------+   |
|  |                    FRONTEND (Vue.js)                         |   |
|  |  +----------+ +----------+ +----------+ +----------+        |   |
|  |  | Login    | | Order    | | Document | | CSV      |        |   |
|  |  | Screen   | | List     | | Viewer   | | Export   |        |   |
|  |  +----------+ +----------+ +----------+ +----------+        |   |
|  |  +----------+ +----------+ +----------+                     |   |
|  |  | Partner  | | Free     | | Output   |                     |   |
|  |  | Master   | | Input    | | Order    |                     |   |
|  |  +----------+ +----------+ +----------+                     |   |
|  +----------------------------+--------------------------------+   |
|                            | REST API / HTTP                      |
|  +----------------------------+--------------------------------+   |
|  |                    BACKEND (Laravel/PHP)                     |   |
|  |                                                              |   |
|  |  +------------+  +------------+  +------------+             |   |
|  |  | Auth       |  | Order      |  | Document   |             |   |
|  |  | Service    |  | Service    |  | Service    |             |   |
|  |  +------------+  +------------+  +------------+             |   |
|  |  +------------+  +------------+  +------------+             |   |
|  |  | OCR        |  | Validation |  | CSV        |             |   |
|  |  | Service    |  | Service    |  | Service    |             |   |
|  |  +------------+  +------------+  +------------+             |   |
|  |  +------------+  +------------+                              |   |
|  |  | Partner    |  | FreeInput  |                              |   |
|  |  | Service    |  | Service    |                              |   |
|  |  +------------+  +------------+                              |   |
|  +----------------------------+--------------------------------+   |
|                            |                                      |
|  +----------------------------+--------------------------------+   |
|  |                    DATABASE (MySQL 8)                        |   |
|  |  +------+ +------+ +------+ +------+ +------+ +------+    |   |
|  |  |users | |orders| |docs  | |ocr   | |partner| |free  |    |   |
|  |  |      | |      | |      | |result| |master | |input |    |   |
|  |  +------+ +------+ +------+ +------+ +------+ +------+    |   |
|  +-------------------------------------------------------------+   |
|                                                                     |
|  +-------------------------------------------------------------+   |
|  |                    EXTERNAL                                  |   |
|  |  +--------------+                    +--------------+        |   |
|  |  | AI OCR API   |                    | File Storage |        |   |
|  |  | (External)   |                    | (PDF/Images) |        |   |
|  |  +--------------+                    +--------------+        |   |
|  +-------------------------------------------------------------+   |
+---------------------------------------------------------------------+
```

## 2.2 Data Flow Architecture

```
                        +--------------+
                        |   User       |
                        |  (Browser)   |
                        +------+-------+
                               |
                    +----------+----------+
                    |   Step 1: UPLOAD    |
                    |   Drag & Drop PDFs  |
                    +----------+----------+
                               |
                               v
                    +----------------------+
                    |   Frontend (Vue.js)  |
                    |   - Validate files   |
                    |   - Preview upload   |
                    +----------+-----------+
                               | POST /api/orders/{id}/documents
                               v
                    +----------------------+
                    |   Backend (Laravel)  |
                    |   - Store PDF files  |
                    |   - Create records   |
                    +----------+-----------+
                               |
                    +----------+----------+
                    |  Step 2: AIOCR       |
                    |  Batch Processing    |
                    +----------+----------+
                               |
                               v
                    +----------------------+
                    |   AI OCR API Call    |
                    |   - Send PDF         |
                    |   - Receive results  |
                    |   - Extract fields   |
                    +----------+-----------+
                               |
                               v
                    +----------------------+
                    |   Result Processing  |
                    |   - Parse OCR data   |
                    |   - Calculate conf.  |
                    |   - Store in DB      |
                    +----------+-----------+
                               |
                    +----------+----------+
                    |  Step 3: VALIDATE    |
                    |  Cross-doc Check     |
                    +----------+----------+
                               |
                               v
                    +----------------------+
                    |   Consistency Check  |
                    |   Invoice vs PL vs BL|
                    |   - Amount match?    |
                    |   - Quantity match?  |
                    |   - Partner match?   |
                    +----------+-----------+
                               |
                    +----------+----------+
                    |  Step 4: REVIEW      |
                    |  User Confirmation   |
                    +----------+----------+
                               |
                               v
                    +----------------------+
                    |   Split View Screen  |
                    |   +--------+-------+ |
                    |   | Image  | Data  | |
                    |   | (Left) |(Right)| |
                    |   +--------+-------+ |
                    |   - Edit fields      |
                    |   - Free input       |
                    |   - Partner master   |
                    +----------+-----------+
                               |
                    +----------+----------+
                    |  Step 5: EXPORT      |
                    |  CSV Download        |
                    +----------+----------+
                               |
                               v
                    +----------------------+
                    |   CSV Generation     |
                    |   - Config columns   |
                    |   - Sort order       |
                    |   - Download CSV     |
                    +----------+-----------+
                               |
                               v
                    +----------------------+
                    |   TOSS System        |
                    |   (CSV Import)       |
                    +----------------------+
```

## 2.3 Component Interaction Diagram

```
+---------+        +---------+        +---------+        +---------+
|  Vue.js |        | Laravel |        | MySQL 8 |        | OCR API |
|  SPA    |        |  API    |        |   DB    |        | (Ext.)  |
+----+----+        +----+----+        +----+----+        +----+----+
     |                  |                   |                  |
     |  1. Login        |                   |                  |
     |----------------->|                   |                  |
     |  2. JWT Token    |                   |                  |
     |<-----------------|                   |                  |
     |                  |                   |                  |
     |  3. Upload PDFs  |                   |                  |
     |----------------->|  4. Store file    |                  |
     |                  |------------------>|                  |
     |                  |  5. Create order  |                  |
     |                  |------------------>|                  |
     |                  |                   |                  |
     |  6. Trigger OCR  |                   |                  |
     |----------------->|  7. Call OCR API  |                  |
     |                  |------------------------------------->|
     |                  |  8. OCR Results   |                  |
     |                  |<-------------------------------------|
     |                  |  9. Save results  |                  |
     |                  |------------------>|                  |
     |                  |                   |                  |
     |  10. Get results |                   |                  |
     |----------------->|  11. Query data   |                  |
     |  12. JSON data   |<------------------|                  |
     |<-----------------|                   |                  |
     |                  |                   |                  |
     |  13. Update data |                   |                  |
     |----------------->|  14. Save edits   |                  |
     |                  |------------------>|                  |
     |                  |                   |                  |
     |  15. Export CSV  |                   |                  |
     |----------------->|  16. Generate CSV |                  |
     |  17. Download    |<------------------|                  |
     |<-----------------|                   |                  |
     |                  |                   |                  |
```

## 2.4 Deployment Architecture

```
+-----------------------------------------------------+
|              DEVELOPMENT ENVIRONMENT                 |
|                                                      |
|  - PHP (Laravel Framework)                           |
|  - Vue.js (Frontend SPA)                             |
|  - MySQL 8                                           |
|  - Local / Docker                                    |
+-----------------------------------------------------+

+-----------------------------------------------------+
|              PRODUCTION ENVIRONMENT                  |
|                                                      |
|  - PHP (Laravel Framework)                           |
|  - Vue.js (Frontend SPA)                             |
|  - MySQL 8                                           |
|  - Web Server (Apache/Nginx)                         |
+-----------------------------------------------------+
```

## 2.5 Security Architecture

```
+-----------------------------------------------------+
|                   SECURITY LAYERS                    |
|                                                      |
|  +----------------------------------------------+   |
|  | Layer 1: Authentication                       |   |
|  | - Login screen (username/password)            |   |
|  | - JWT Token based authentication              |   |
|  | - Session management                          |   |
|  +----------------------------------------------+   |
|                                                      |
|  +----------------------------------------------+   |
|  | Layer 2: Authorization                        |   |
|  | - Role-based access control                   |   |
|  | - API endpoint protection                     |   |
|  +----------------------------------------------+   |
|                                                      |
|  +----------------------------------------------+   |
|  | Layer 3: Data Protection                      |   |
|  | - HTTPS encryption                            |   |
|  | - File upload validation                      |   |
|  | - SQL injection prevention (Eloquent ORM)     |   |
|  +----------------------------------------------+   |
+-----------------------------------------------------+
```


---

# 03. User Flows (Luồng người dùng)

## 3.1 Luồng Người dùng Chính -- Tổng quát (Main User Flow)

```
                          +-------------+
                          |   BẮT ĐẦU  |
                          | (Người dùng|
                          |  mở hệ     |
                          |  thống)    |
                          +------+------+
                                 |
                                 v
                          +-------------+
                     +--->|  Đăng nhập  |
                     |    |  (Login)    |
                     |    +------+------+
                     |           | Nhập tên/mật khẩu
                     |           v
                     |    +-------------+      +-------------+
                     |    | Xác thực    +--No->| Hiển thị    |
                     |    | OK?         |      | lỗi đăng    |
                     |    +------+------+      | nhập        |
                     |           |             +-------------+
                     |           | Yes
                     |           v
                     |    +-------------+
                     |    | Danh sách   |
                     |    | Đơn hàng    |
                     |    | (Case List) |
                     |    +------+------+
                     |           |
                     |     +-----+---------------------+
                     |     |     |                     |
                     |     v     v                     v
                     |   +---+ +--------+  +-----------+--+
                     |   |Tạo| |Chọn    |  | Quản lý     |
                     |   |Mới| |Tồn tại |  | Đối tác     |
                     |   +-+-+ +---+----+  +-----------+--+
                     |     |       |
                     |     v       v
                     |   +-------------+
                     |   |  TẢI LÊN   |
                     |   |  PDF Files |
                     |   |  (Kéo thả) |
                     |   +------+------+
                     |          |
                     |          v
                     |   +-------------+
                     |   | XỬ LÝ      |
                     |   | AI OCR     |
                     |   | (Tự động)  |
                     |   +------+------+
                     |          |
                     |          v
                     |   +-------------+
                     |   | KIỂM TRA   |
                     |   | NHẤT QUÁN  |
                     |   | (Tự động)  |
                     |   +------+------+
                     |          |
                     |          v
                     |   +-----------------------------+
                     |   | TRÌNH XEM TÀI LIỆU        |
                     |   | +-----------+-----------+  |
                     |   | | Hình PDF  | Dữ liệu   |  |
                     |   | | (Trái)    | có cấu trúc|  |
                     |   | |           | (Phải)     |  |
                     |   | +-----------+-----------+  |
                     |   |                             |
                     |   | Mã màu:                    |
                     |   | [!] Độ tin cậy thấp         |
                     |   | [~] Cần xem xét             |
                     |   | [+] Độ tin cậy cao          |
                     |   +-------+---------------------+
                     |           |
                     |      +----+--------+-------+
                     |      |    |        |       |
                     |      v    v        v       v
                     |   +----++----+ +------+ +- -----+
                     |   |Gộp ||Chọn| |Nhập  | |Quản lý|
                     |   |Khối||Lọc | |Tự do | |Đối tác|
                     |   +--+-++--+-+ +--+---+ +-------+
                     |      |    |       |
                     |      v    v       v
                     |   +-------------+
                     |   | LƯU         |
                     |   | Thay đổi    |
                     |   +------+------+
                     |          |
                     |          v
                     |   +-------------+
                     |   | THỨ TỰ     |
                     |   | XUẤT       |
                     |   | (Kéo thả)  |
                     |   +------+------+
                     |          |
                     |          v
                     |   +-------------+
                     |   | XUẤT CSV   |
                     |   | & Tải về   |
                     |   +------+------+
                     |          |
                     |          v
                     |   +-------------+
                     |   | Nhập vào   |
                     |   | TOSS       |
                     |   | (Thủ công) |
                     |   +------+------+
                     |          |
                     |          v
                     |   +-------------+
                     +---| HOÀN THÀNH |
                         +-------------+
```

## 3.2 Luồng Tải lên (Upload Flow)

```
+-------------------------------------------------------------+
|                    LUỒNG TẢI LÊN (UPLOAD FLOW)              |
|                                                              |
|  +----------+                                                |
|  | Người    |                                                |
|  | dùng     |                                                |
|  | chọn     |                                                |
|  | Đơn hàng |                                                |
|  +----+-----+                                                |
|       |                                                      |
|       v                                                      |
|  +----------------------------------------+                 |
|  |     VÙNG TẢI LÊN (Kéo thả)             |                 |
|  | +------------------------------------+ |                 |
|  | |                                    | |                 |
|  | |    [DIR] Kéo file vào đây hoặc Click  | |                 |
|  | |                                    | |                 |
|  | |    Chấp nhận: .pdf                 | |                 |
|  | |    Tối đa: ~10 PDF                 | |                 |
|  | +------------------------------------+ |                 |
|  +----------------------------------------+                 |
|       |                                                      |
|       | Files được thả vào                                   |
|       v                                                      |
|  +------------------+                                        |
|  | Xác thực file    |                                       |
|  | - Định dạng PDF? +--No--> Thông báo lỗi                  |
|  | - Kích thước OK? |                                       |
|  | - Số lượng OK?   |                                       |
|  +--------+---------+                                        |
|           | Yes                                               |
|           v                                                  |
|  +------------------+                                        |
|  | Xem trước File   |                                       |
|  | +--------------+ |                                       |
|  | | invoice.pdf  | |                                       |
|  | | pl.pdf       | |                                       |
|  | | bl.pdf       | |                                       |
|  | +--------------+ |                                       |
|  +--------+---------+                                        |
|           |                                                  |
|           v                                                  |
|  +------------------+                                        |
|  | Tải lên Máy chủ +-- Lỗi --> Thử lại / Hiển thị lỗi      |
|  +--------+---------+                                        |
|           | Thành công                                       |
|           v                                                  |
|  +------------------+                                        |
|  | So khớp Mẫu     |                                        |
|  | Biểu mẫu        |                                        |
|  | (Template)      |                                        |
|  +--------+---------+                                        |
|           |                                                  |
|           v                                                  |
|  +------------------+                                        |
|  | Kích hoạt Xử lý |                                        |
|  | AIOCR Hàng loạt |                                        |
|  +------------------+                                        |
+-------------------------------------------------------------+
```

## 3.3 Luồng Xử lý AIOCR (AIOCR Processing Flow)

```
+-------------------------------------------------------------+
|              LUỒNG XỬ LÝ AIOCR                               |
|                                                              |
|  +-------------+                                            |
|  | PDF Files   |                                            |
|  | đã tải lên |                                            |
|  +------+------+                                            |
|         |                                                    |
|         v                                                    |
|  +-----------------------------------------+               |
|  | Cho từng PDF:                            |               |
|  |                                          |               |
|  | +---------+ +---------+ +-----+          |               |
|  | |Invoice  | |   PL    | | B/L |          |               |
|  | |  PDF    | |   PDF   | | PDF |          |               |
|  | +----+----+ +----+----+ +--+--+          |               |
|  |      |           |         |              |               |
|  |      v           v         v              |               |
|  | +----------------------------------+      |               |
|  | |    Engine AI OCR                 |      |               |
|  | |  - Trích xuất text & cấu trúc   |      |               |
|  | |  - Nhận dạng loại trường        |      |               |
|  | |  - Tính điểm tin cậy            |      |               |
|  | |  - Tự động gộp khối gần nhau   |      |               |
|  | +---------+------------------------+      |               |
|  +-----------+-------------------------------+               |
|              |                                              |
|              v                                              |
|  +-----------------------------------------+               |
|  | Dữ liệu Trích xuất (mỗi tài liệu):     |               |
|  |                                          |               |
|  | Invoice:                                 |               |
|  | +-- Số HĐ (Inv No.)     [conf: 0.95] [+]|               |
|  | +-- Ngày (Date)          [conf: 0.92] [+]|               |
|  | +-- Người bán (Seller)   [conf: 0.88] [+]|               |
|  | +-- Người mua (Buyer)    [conf: 0.91] [+]|               |
|  | +-- Tên SP (Product)     [conf: 0.75] [~]|               |
|  | +-- Số lượng (Qty)       [conf: 0.98] [+]|               |
|  | +-- Đơn giá (Unit Price) [conf: 0.96] [+]|               |
|  | +-- Thành tiền (Amount)  [conf: 0.99] [+]|               |
|  |                                          |               |
|  | PL:                                      |               |
|  | +-- Số PL (PL No.)       [conf: 0.93] [+]|               |
|  | +-- Trọng lượng (Weight) [conf: 0.87] [+]|               |
|  | +-- Kích thước (Dims)    [conf: 0.82] [~]|               |
|  |                                          |               |
|  | B/L:                                     |               |
|  | +-- Số B/L (B/L No.)     [conf: 0.94] [+]|               |
|  | +-- Người gửi (Shipper)  [conf: 0.90] [+]|               |
|  | +-- Người nhận (Consigne)[conf: 0.89] [+]|               |
|  | +-- Tàu (Vessel)         [conf: 0.96] [+]|               |
|  +------------------+-----------------------+               |
|                     |                                        |
|                     v                                        |
|  +------------------------------------------+               |
|  | KIỂM TRA NHẤT QUÁN GIỮA TÀI LIỆU       |               |
|  |                                          |               |
|  | Kiểm tra 1: Số tiền (Amounts)           |               |
|  |   Tổng Invoice == Tổng PL?              |               |
|  |                                          |               |
|  | Kiểm tra 2: Số lượng (Quantities)       |               |
|  |   SL Invoice == SL PL?                  |               |
|  |                                          |               |
|  | Kiểm tra 3: Thông tin Đối tác (Partner) |               |
|  |   Người bán Invoice == Người gửi B/L?   |               |
|  |                                          |               |
|  | Kiểm tra 4: Thông tin Vận chuyển        |               |
|  |   Trọng lượng PL == Trọng lượng B/L?    |               |
|  |                                          |               |
|  | Kết quả:                                 |               |
|  | [OK] Khớp (Match)  [!] Không khớp (Mismatch)|               |
|  +------------------+-----------------------+               |
|                     |                                        |
|                     v                                        |
|  +------------------------------------------+               |
|  | XÁC THỰC ĐA TÍN HIỆU (Multi-signal)     |               |
|  |                                          |               |
|  | Tín hiệu 1: Điểm tin cậy OCR             |               |
|  | Tín hiệu 2: Xác thực định dạng           |               |
|  | Tín hiệu 3: Nhất quán giữa tài liệu      |               |
|  |                                          |               |
|  | Kết quả kết hợp:                         |               |
|  | [+] Xanh:  Tin cậy cao + Hợp lệ + Nhất quán|              |
|  | [~] Vàng: Tin cậy cao + Vấn đề định dạng  |               |
|  | [!] Đỏ:   Tin cậy thấp + Không hợp lệ     |               |
|  +------------------------------------------+               |
+-------------------------------------------------------------+
```

## 3.4 Luồng Xem lại Tài liệu (Document Review Flow) -- Split View

```
+-------------------------------------------------------------+
|           LUỒNG XEM LẠI TÀI LIỆU                           |
|                                                              |
|  +----------------------------------------------+           |
|  |        MÀN HÌNH TRÌNH XEM TÀI LIỆU          |           |
|  |                                               |           |
|  | +---------------------+---------------------+|           |
|  | |                     | Dữ liệu Cấu trúc    ||           |
|  | |  Hình ảnh PDF       |                      ||           |
|  | |  (Bảng Trái)        | Số HĐ: RS-344       ||           |
|  | |                     | Ngày: 2026/03/15    ||           |
|  | | [Vùng tô sáng       | Người bán: ABC [+]   ||           |
|  | |  khớp với trường    | Thành tiền: $5000 [+]||           |
|  | |  được chọn]         | Các mục:            ||           |
|  | |                     |   1. FLOOR CHAIR [+] ||           |
|  | |                     |   2. CUSHION [+]     ||           |
|  | |                     |                      ||           |
|  | +---------------------+---------------------+|           |
|  |                                               |           |
|  | Điều hướng:                                   |           |
|  | +-------+ +-------+ +-------+ +----------+  |           |
|  | |< Trước| |Sau >  | |Trang #| |Cập nhật  |  |           |
|  | +-------+ +-------+ +-------+ +----------+  |           |
|  +----------------------------------------------+           |
|                                                              |
|  Hành động Người dùng:                                      |
|                                                              |
|  +-- Hành động 1: Chọn trường bên phải --------+           |
|  | -> Tô sáng vùng tương ứng bên trái           |           |
|  | -> Hiển thị điểm tin cậy                     |           |
|  +---------------------------------------------+           |
|                                                              |
|  +-- Hành động 2: Click vùng trên hình ảnh ----+           |
|  | -> Chọn trường tương ứng bên phải            |           |
|  | -> Bật chế độ chỉnh sửa nội tuyến (inline)   |           |
|  +---------------------------------------------+           |
|                                                              |
|  +-- Hành động 3: Chỉnh sửa giá trị trường ---+           |
|  | -> Xuất hiện trường nhập text                |           |
|  | -> Cập nhật màu tin cậy                      |           |
|  | -> Lưu bằng nút Cập nhật                     |           |
|  +---------------------------------------------+           |
|                                                              |
|  +-- Hành động 4: Gộp khối (Block Merge) -----+           |
|  | -> Click khối A + Shift+click khối B, C     |           |
|  | -> Bấm "Gộp" -> tạo trường đã gộp            |           |
|  | -> merged_children JSON lưu các khối gốc     |           |
|  +---------------------------------------------+           |
|                                                              |
|  +-- Hành động 5: Điều hướng tài liệu --------+            |
|  | Nút Trước/Sau duyệt qua:                   |            |
|  | Invoice p1 -> Invoice p2 -> ... -> PL -> B/L   |            |
|  +---------------------------------------------+           |
|                                                              |
|  +-- Hành động 6: Chọn lọc trường (Select) ---+            |
|  | -> Checkbox bên cạnh mỗi trường              |            |
|  | -> Bỏ chọn = loại trừ khỏi xuất CSV          |            |
|  +---------------------------------------------+           |
+-------------------------------------------------------------+
```

## 3.5 Luồng Xuất CSV (CSV Export Flow)

```
+-------------------------------------------------------------+
|                LUỒNG XUẤT CSV                               |
|                                                              |
|  +--------------+                                           |
|  | Người dùng   |                                           |
|  | hoàn thành   |                                           |
|  | xem xét      |                                           |
|  | (review)     |                                           |
|  +------+-------+                                           |
|         |                                                    |
|         v                                                    |
|  +----------------------------------------+                 |
|  |      MÀN HÌNH THỨ TỰ XUẤT              |                 |
|  |                                         |                 |
|  | Bước 1: Chọn dòng (mục hàng):          |                 |
|  | +------------------------------------+ |                 |
|  | | [x] Mục #1: FLOOR CHAIR  SL: 100  | |                 |
|  | | [ ] Mục #2: CUSHION      SL: 200  | |                 |
|  | | [x] Mục #3: BACK REST    SL: 50   | |                 |
|  | | [Chọn tất cả] [Bỏ chọn tất cả]    | |                 |
|  | +------------------------------------+ |                 |
|  |                                         |                 |
|  | Bước 2: Kéo thả sắp xếp cột CSV:       |                 |
|  | +------------------------------------+ |                 |
|  | | ::: 1. 発注番号 (Mã ĐH)             | |                 |
|  | | ::: 2. 品名 (Tên SP)                 | |                 |
|  | | ::: 3. 数量 (Số lượng)               | |                 |
|  | | ::: 4. 単価 (Đơn giá)                | |                 |
|  | | ::: 5. 金額 (Thành tiền)             | |                 |
|  | | ::: 6. 取引先コード (Mã ĐT)          | |                 |
|  | +------------------------------------+ |                 |
|  |                                         |                 |
|  | [ ] Thêm/Loại bỏ cột                     |                 |
|  +-------------------+---------------------+                 |
|                      |                                       |
|                      v                                       |
|  +----------------------------------------+                 |
|  |      XEM TRƯỚC & XÁC NHẬN              |                 |
|  |                                         |                 |
|  | Xem trước CSV:                          |                 |
|  | +------------------------------------+ |                 |
|  | | Mã ĐH, Tên SP, SL, Đơn giá,...    | |                 |
|  | | RS-344, FLOOR CHAIR, 100, 5000,... | |                 |
|  | | RS-344, BACK REST, 50, 8000,...    | |                 |
|  | +------------------------------------+ |                 |
|  |                                         |                 |
|  | [Xác nhận]  [Quay lại]                 |                 |
|  +-------------------+---------------------+                 |
|                      | Xác nhận                              |
|                      v                                       |
|  +----------------------------------------+                 |
|  |      TẢI CSV                            |                 |
|  |                                         |                 |
|  | Định dạng: UTF-8 CSV (Tương thích TOSS)|                 |
|  | Tên file: order_{発注番号}_{date}.csv   |                 |
|  |                                         |                 |
|  | +------------------+                    |                 |
|  | | [SAVE] Tải CSV       |                    |                 |
|  | +------------------+                    |                 |
|  +-------------------+---------------------+                 |
|                      |                                       |
|                      v                                       |
|  +----------------------------------------+                 |
|  |      NHẬP VÀO TOSS                      |                 |
|  | (Thủ công - Người dùng import CSV)      |                 |
|  +----------------------------------------+                 |
+-------------------------------------------------------------+
```

## 3.6 Luồng Tự động Truy xuất Đối tác (Partner Master Auto-Retrieval Flow)

```
+-------------------------------------------------------------+
|         LUỒNG ĐỐI TÁC (PARTNER MASTER FLOW)                |
|                                                              |
|  +------------------+                                       |
|  | OCR trích xuất   |                                       |
|  | tên đối tác      |                                       |
|  | từ Invoice       |                                       |
|  +--------+---------+                                       |
|           |                                                  |
|           v                                                  |
|  +--------------------------------------+                   |
|  | Tìm kiếm trong Quản lý Đối tác       |                   |
|  |                                      |                   |
|  | OCR: "ABC Trading Co., Ltd."        |                   |
|  |        |                             |                   |
|  |        v                             |                   |
|  | +-------------------------------+   |                   |
|  | | Bảng Quản lý Đối tác         |   |                   |
|  | | +--------+--------+---------+|   |                   |
|  | | | Mã     | Tên    | Địa chỉ ||   |                   |
|  | | +--------+--------+---------+|   |                   |
|  | | | P001   | ABC... | Tokyo   ||   |                   |
|  | | | P002   | XYZ... | Osaka   ||   |                   |
|  | | +--------+--------+---------+|   |                   |
|  | +-------------------------------+   |                   |
|  +----------+---------------------------+                   |
|             |                                               |
|     +-------+--------+                                      |
|     |                |                                      |
|     v                v                                      |
|  +----------+  +-----------+                                |
|  | TÌM THẤY |  | KHÔNG    |                                |
|  | (FOUND)  |  | TÌM THẤY |                                |
|  |          |  | (NOT FND)|                                |
|  | Tự động  |  |           |                                |
|  | điền:    |  | Lựa chọn  |                                |
|  | - Mã     |  | 1: Tạo    |                                |
|  | - Tên    |  | Đối tác   |                                |
|  | - Địa chỉ|  | mới       |                                |
|  | - ĐT     |  |           |                                |
|  |          |  | Lựa chọn  |                                |
|  |          |  | 2: Nhập   |                                |
|  |          |  | tay       |                                |
|  +----------+  +-----------+                                |
+-------------------------------------------------------------+
```

## 3.7 Luồng Gộp Khối (Block Grouping Flow) -- MỚI

```
+-------------------------------------------------------------+
|         LUỒNG GỘP KHỐI (BLOCK GROUPING FLOW)               |
|                                                              |
|  +--------------+                                           |
|  | OCR trả về   |                                           |
|  | các khối     |                                           |
|  | riêng lẻ     |                                           |
|  +------+-------+                                           |
|         |                                                    |
|         v                                                    |
|  +---------------------------+                              |
|  | Tự động gộp (Auto-merge) |                              |
|  | +-----------------------+ |                              |
|  | | Phân cụm theo khoảng  | |                              |
|  | | cách (proximity)       | |                              |
|  | | Heuristic cùng dòng   | |                              |
|  | | Kết quả: ~80% đúng    | |                              |
|  | +-----------+-----------+ |                              |
|  +-------------+-------------+                              |
|                |                                             |
|                v                                             |
|  +---------------------------+                              |
|  | Người dùng xem xét       |                              |
|  | kết quả gộp              |                              |
|  +-----+----------+---------+                              |
|        |          |                                         |
|    Đúng          Sai                                        |
|        |          |                                         |
|        v          v                                         |
|  +--------+ +---------------------+                        |
|  | Tiếp   | | Điều chỉnh thủ công |                        |
|  | tục    | | (Manual override)   |                        |
|  +--------+ +----------+----------+                        |
|                        |                                    |
|                   +----+-----+                              |
|                   |          |                              |
|                   v          v                              |
|             +--------+ +--------+                          |
|             | Gộp    | | Tách   |                          |
|             | (Merge)| | (Split)|                          |
|             +---+----+ +---+----+                          |
|                 |          |                                |
|                 v          v                                |
|             +------------------+                            |
|             | Cập nhật Dữ liệu|                            |
|             | merged_children  |                            |
|             | JSON trong DB    |                            |
|             +--------+---------+                            |
|                      |                                      |
|                      v                                      |
|             +------------------+                            |
|             | Lưu & Tiếp tục  |                            |
|             +------------------+                            |
+-------------------------------------------------------------+
```

## 3.8 Luồng So khớp Mẫu Biểu mẫu (Template Matching Flow) -- MỚI

```
+-------------------------------------------------------------+
|    LUỒNG SO KHỚP MẪU BIỂU MẪU (TEMPLATE MATCHING FLOW)     |
|                                                              |
|  +--------------+                                           |
|  | Người dùng   |                                           |
|  | tải lên PDF  |                                           |
|  +------+-------+                                           |
|         |                                                    |
|         v                                                    |
|  +---------------------------+                              |
|  | Nhận dạng loại tài liệu  |                              |
|  | (Invoice / PL / B/L)     |                              |
|  +-------------+-------------+                              |
|                |                                             |
|                v                                             |
|  +---------------------------+                              |
|  | Xác định đối tác (partner)|                              |
|  | từ đơn hàng (order)       |                              |
|  +-------------+-------------+                              |
|                |                                             |
|                v                                             |
|  +---------------------------+                              |
|  | Tìm kiếm mẫu biểu mẫu    |                              |
|  | (Search template)         |                              |
|  | partner_id + doc_type     |                              |
|  | + is_active               |                              |
|  +-------------+-------------+                              |
|                |                                             |
|          +-----+-----+                                      |
|          |           |                                      |
|          v           v                                      |
|     +--------+  +-----------+                               |
|     | Tìm    |  | Không     |                               |
|     | thấy   |  | tìm thấy  |                               |
|     +---+----+  +-----+-----+                               |
|         |              |                                     |
|         v              v                                     |
|  +------------+  +-------------+                            |
|  | Dùng mẫu  |  | Dùng mẫu   |                            |
|  | riêng đối |  | chung      |                            |
|  | tác       |  | (generic)  |                            |
|  +-----+------+  +------+------+                            |
|        |                |                                    |
|        +--------+-------+                                   |
|                 |                                            |
|                 v                                            |
|  +---------------------------+                              |
|  | Áp dụng vùng (region)     |                              |
|  | đến trích xuất OCR        |                              |
|  |                           |                              |
|  | - Cắt vùng theo mẫu      |                              |
|  | - Gửi từng vùng đến OCR  |                              |
|  | - Xác thực theo quy tắc  |                              |
|  +-------------+-------------+                              |
|                |                                             |
|                v                                             |
|  +---------------------------+                              |
|  | Kết quả OCR có cấu trúc  |                              |
|  | với điểm tin cậy          |                              |
|  +---------------------------+                              |
+-------------------------------------------------------------+
```

## 3.9 Sơ đồ Trạng thái -- Vòng đời Đơn hàng (Order Lifecycle)

```
                     +-------------+
                     |  ĐÃ TẠO    |
                     | (CREATED)  |
                     | Tạo mới    |
                     | Đơn hàng   |
                     +------+-----+
                            | Tải lên PDF
                            v
                     +-------------+
                     | ĐÃ TẢI LÊN |
                     | (UPLOADED) |
                     | PDF đã     |
                     | tải lên    |
                     +------+-----+
                            | Kích hoạt OCR
                            v
                     +-------------+
                     | ĐANG XỬ LÝ |
                     |(PROCESSING)|
                     | AIOCR đang |
                     | xử lý      |
                     +------+-----+
                            | OCR hoàn tất
                            v
                     +-------------+         +-------------+
                     | XEM XÉT    +-------->| ĐANG CHỈNH |
                     | (REVIEW)   |<--------| SỬA        |
                     | Chờ người  |         | (EDITING)  |
                     | dùng xem   |         | Đang chỉnh |
                     +------+-----+         +-------------+
                            | Đã xác nhận
                            v
                     +-------------+
                     | ĐÃ XÁC NHẬN|
                     | (CONFIRMED)|
                     | Người dùng |
                     | xác nhận   |
                     +------+-----+
                            | Xuất CSV
                            v
                     +-------------+
                     | ĐÃ XUẤT    |
                     | (EXPORTED) |
                     | CSV đã     |
                     | tải về     |
                     +-------------+
```


---

# 04. Wireframes (Thiết kế màn hình)

## 4.1 Màn hình Đăng nhập (Login Screen)

```
+--------------------------------------------------------------+
|                                                              |
|                                                              |
|                                                              |
|                    +---------------------------+             |
|                    |    AIOCR System           |             |
|                    |    貿易書類&受注FAX        |             |
|                    |                           |             |
|                    |  +---------------------+  |             |
|                    |  | Username            |  |             |
|                    |  +---------------------+  |             |
|                    |                           |             |
|                    |  +---------------------+  |             |
|                    |  | Password            |  |             |
|                    |  +---------------------+  |             |
|                    |                           |             |
|                    |  +---------------------+  |             |
|                    |  |     ログイン         |  |             |
|                    |  |    (Đăng nhập)       |  |             |
|                    |  +---------------------+  |             |
|                    |                           |             |
|                    +---------------------------+             |
|                                                              |
|                                                              |
+--------------------------------------------------------------+
```

## 4.2 Màn hình Danh sách Đơn hàng (Order List Screen)

```
+----------------------------------------------------------------------+
| AIOCR System  |  注文一覧 (Danh sách Đơn hàng)  |  ログアウト (Thoát)|
+----------------------------------------------------------------------+
|                                                                      |
|  +-------------------------+  +-------------------------+            |
|  | + 新規注文 (Tạo mới)    |  | [SEARCH] 検索 (Tìm kiếm)      |            |
|  +-------------------------+  | [_____________________] |            |
|                               +-------------------------+            |
|                                                                      |
|  +---------------------------------------------------------------+  |
|  | 発注番号   | 取引先      | 書類数 | ステータス  | 登録日       |  |
|  | (Mã ĐH)   | (Đối tác)   | (TL)  | (Tr.thái)  | (Ngày tạo)   |  |
|  |------------+-------------+--------+------------+--------------|  |
|  | RS-FB-344  | ABC Corp    |   3    | [LIST] Review  | 2026/03/15   |  |
|  | RS-FB-345  | XYZ Ltd     |   5    | [OK] Xác nhận| 2026/03/14   |  |
|  | RS-FB-346  | DEF Inc     |   2    | [WAIT] Xử lý   | 2026/03/13   |  |
|  | RS-FB-347  | GHI Co      |   4    | [SENT] Đã xuất | 2026/03/12   |  |
|  | ...        | ...         |  ...   | ...        | ...          |  |
|  +---------------------------------------------------------------+  |
|                                                                      |
|  +---------------------------------------------------------------+  |
|  |  < 1  2  3  4  5 >                      Hiển thị 1-20 / 85    |  |
|  +---------------------------------------------------------------+  |
|                                                                      |
|  +------------------+                                                |
|  | 取引先マスタ      |  <-- Liên kết tới Quản lý Đối tác            |
|  | (Quản lý Đối tác)|                                                |
|  +------------------+                                                |
+----------------------------------------------------------------------+
```

## 4.3 Màn hình Chi tiết Đơn hàng -- Tải lên (Upload Screen)

```
+----------------------------------------------------------------------+
| AIOCR System | RS-FB-344 | Tải lên (Upload)                        |
+----------------------------------------------------------------------+
|                                                                      |
| 発注番号: RS-FB-344    取引先: ABC Corp    ステータス: Upload        |
|                                                                      |
|  +---------------------------------------------------------------+  |
|  |                                                                |  |
|  |              +----------------------------+                   |  |
|  |              |                            |                   |  |
|  |              |    [DIR]                       |                   |  |
|  |              |                            |                   |  |
|  |              |   Kéo thả file vào đây     |                   |  |
|  |              |   hoặc Click để duyệt       |                   |  |
|  |              |                            |                   |  |
|  |              |   Chấp nhận: .pdf          |                   |  |
|  |              |                            |                   |  |
|  |              +----------------------------+                   |  |
|  |                                                                |  |
|  +---------------------------------------------------------------+  |
|                                                                      |
| File đã tải lên:                                                     |
|  +---------------------------------------------------------------+  |
|  | [DOC] invoice_RS-FB-344.pdf     (Invoice)   3 trang  |  [OK]  [DEL]   |  |
|  | [DOC] packinglist_RS-FB-344.pdf (PL)        2 trang  |  [OK]  [DEL]   |  |
|  | [DOC] bl_RS-FB-344.pdf          (B/L)       2 trang  |  [OK]  [DEL]   |  |
|  +---------------------------------------------------------------+  |
|                                                                      |
| Tự động nhận dạng loại tài liệu: Invoice, PL, B/L                   |
|                                                                      |
|  +----------------------------+  +----------------------------+      |
|  | Bắt đầu xử lý AIOCR       |  | Quay lại (Back)            |      |
|  +----------------------------+  +----------------------------+      |
+----------------------------------------------------------------------+
```

## 4.4 Màn hình Trình xem Tài liệu (Document Viewer) -- Split View -- MÀN HÌNH CHÍNH

```
+----------------------------------------------------------------------+
| AIOCR System | RS-FB-344 | Trình xem Tài liệu    |  <-- Quay lại   |
+----------------------------------------------------------------------+
|                                                                      |
| Tài liệu: invoice_RS-FB-344.pdf   Trang: 1/3   Loại: Invoice       |
|                                                                      |
| +-----------------------------+------------------------------------+ |
| |                             |  Kết quả OCR - Invoice             | |
| |                             |                                    | |
| |    +------------------+     |  +- Tiêu đề Hóa đơn ------------+ | |
| |    |                  |     |  | Số HĐ:   [RS-FB-344    ] [+]  | | |
| |    |                  |     |  | Ngày:    [2026/03/15  ] [+]  | | |
| |    |  Hình ảnh PDF    |     |  | Người bán:[ABC Corp   ] [+]  | | |
| |    |                  |     |  | Người mua:[XYZ Ltd    ] [+]  | | |
| |    |  (Bản scan       |     |  +-------------------------------+ | |
| |    |   gốc/hình ảnh)  |     |                                    | |
| |    |                  |     |  +- Mục hàng (Items) ----------+  | |
| |    |                  |     |  | #1 FLOOR CHAIR               |  | |
| |    |  +----------+   |     |  |    Mã:  XY-CR-7515-C    [+]  |  | |
| |    |  | Vùng     |   |     |  |    SL:  100              [+]  |  | |
| |    |  | tô sáng  |   |     |  |    Đơn giá: 5,000       [+]  |  | |
| |    |  | khớp với |   |     |  |    Thành tiền: 500,000   [+]  |  | |
| |    |  | trường   |   |     |  |                               |  | |
| |    |  | được chọn |   |     |  | #2 CUSHION                   |  | |
| |    |  +----------+   |     |  |    Mã:  XY-CS-7520-B    [+]  |  | |
| |    |                  |     |  |    SL:  200              [+]  |  | |
| |    |                  |     |  |    Đơn giá: 3,000       [!]  |  | |
| |    |                  |     |  |    Thành tiền: 600,000   [!]  |  | |
| |    +------------------+     |  +-------------------------------+ | |
| |                             |                                    | |
| |                             |  +- Kiểm tra Nhất quán ----------+| |
| |                             |  | Tổng Invoice: 1,100,000       || |
| |                             |  | Tổng PL:     1,100,000    [OK]  || |
| |                             |  | SL khớp:                [OK]   || |
| |                             |  | Đối tác khớp:           [!]   || |
| |                             |  +--------------------------------+| |
| +-----------------------------+------------------------------------+ |
|                                                                      |
| +------------------------------------------------------------------+ |
| |  Điều hướng:                                                     | |
| |  < [Inv p1 [+]] [Inv p2 [+]] [Inv p3 [~]] [PL p1 [!]] [B/L [+]] >  | |
| |                                                                  | |
| |  Chú thích màu: [+] Tin cậy cao (>0.8) [!] Tin cậy thấp (<0.8)  | |
| +------------------------------------------------------------------+ |
|                                                                      |
| +------------------+ +------------------+ +------------------------+ |
| | Cập nhật (Lưu)  | | Nhập Tự do       | | Thứ tự Xuất (CSV)     | |
| +------------------+ +------------------+ +------------------------+ |
+----------------------------------------------------------------------+
```

### 4.4.1 Chi tiết: Tương tác Gộp Khối (Block Grouping Interaction)

```
+----------------------------------------------------------------------+
| CHẾ ĐỘ GỘP KHỐI (BLOCK MERGE MODE)                                  |
+----------------------------------------------------------------------+
|                                                                      |
| Trạng thái BAN ĐẦU -- OCR trả về các khối riêng lẻ:                 |
| +-----------------------------+------------------------------------+ |
| |                             |  Các khối OCR chưa gộp:           | |
| |    +------+ +------+       |                                    | |
| |    | ABC  | | Corp |       |  [ ] khối 1: "ABC"   (conf: 0.95) | |
| |    +------+ +------+       |  [ ] khối 2: " Corp" (conf: 0.90) | |
| |                             |                                    | |
| +-----------------------------+------------------------------------+ |
|                                                                      |
| Bước 1: Người dùng chọn nhiều khối (Shift+click):                  |
| +-----------------------------+------------------------------------+ |
| |                             |  Các khối OCR chưa gộp:           | |
| |    +======+ +======+       |                                    | |
| |    | ABC  | | Corp |       |  [x] khối 1: "ABC"   <- đã chọn   | |
| |    +======+ +======+       |  [x] khối 2: " Corp" <- đã chọn   | |
| |                             |                                    | |
| +-----------------------------+------------------------------------+ |
|                                                                      |
| Bước 2: Bấm nút "Gộp" (Merge):                                     |
| +-----------------------------+------------------------------------+ |
| |                             |  Trường đã gộp:                   | |
| |    +----------------+       |                                    | |
| |    | ABC Corp       |       |  Người bán: [ABC Corp  ] [+]      | |
| |    +----------------+       |      -> Khối con: "ABC" + " Corp"   | |
| |                             |    [Gộp] [Tách]                   | |
| +-----------------------------+------------------------------------+ |
|                                                                      |
| Nút thao tác:  [Gộp các khối đã chọn]  [Tách trường này]           |
+----------------------------------------------------------------------+
```

### 4.4.2 Chi tiết: Xử lý Chọn lọc Trường (Selective Field Processing)

```
+----------------------------------------------------------------------+
| CHỌN LỌC TRƯỜNG ĐỂ XUẤT (SELECTIVE FIELD PROCESSING)                |
+----------------------------------------------------------------------+
|                                                                      |
| Cấp 1 -- Chọn theo tài liệu:                                         |
| +---------------------------------------------------------------+   |
| | [x] Invoice (Hóa đơn)     -> Bao gồm trong xem xét            |   |
| | [x] PL (Danh sách đóng gói) -> Bao gồm trong xem xét         |   |
| | [ ] B/L (Vận đơn)         -> Bỏ qua                          |   |
| +---------------------------------------------------------------+   |
|                                                                      |
| Cấp 2 -- Chọn theo trường:                                           |
| +---------------------------------------------------------------+   |
| | [x] Số HĐ (Invoice No): RS-FB-344         -> Xuất            |   |
| | [x] Ngày (Date): 2026/03/15                -> Xuất            |   |
| | [ ] Người mua (Buyer): XYZ Ltd             -> Bỏ qua          |   |
| | [x] Người bán (Seller): ABC Corp           -> Xuất            |   |
| +---------------------------------------------------------------+   |
|                                                                      |
| Cấp 3 -- Chọn theo trường con (bảng mục hàng):                      |
| +---------------------------------------------------------------+   |
| | Mục #1: FLOOR CHAIR                                          |   |
| |   [x] Tên SP (Tên sản phẩm): FLOOR CHAIR   -> Xuất           |   |
| |   [x] Mã SP (Mã sản phẩm):  XY-CR-7515-C  -> Xuất           |   |
| |   [ ] Đơn giá (Giá đơn vị): 5,000          -> Bỏ qua         |   |
| |   [x] Số lượng: 100                         -> Xuất           |   |
| +---------------------------------------------------------------+   |
|                                                                      |
| +------------------+ +--------------------+                          |
| | Chọn tất cả      | | Bỏ chọn tất cả     |                          |
| +------------------+ +--------------------+                          |
+----------------------------------------------------------------------+
```

### 4.4.3 Chi tiết: Biểu ngữ Tóm tắt Sau OCR (Post-OCR Summary Banner)

```
+----------------------------------------------------------------------+
| BIỂU NGỮ TÓM TẮT KẾT QUẢ OCR                                        |
+----------------------------------------------------------------------+
|                                                                      |
| +---------------------------------------------------------------+   |
| | [STATS] Kết quả OCR: 45 trường đã trích xuất                      |   |
| |                                                               |   |
| | [+] 38 độ tin cậy cao    [~] 5 cần xem xét    [!] 2 cần sửa    |   |
| |                                                               |   |
| | -> Click để nhảy tới vấn đề đầu tiên                          |   |
| +---------------------------------------------------------------+   |
|                                                                      |
| Thanh điều hướng tài liệu với trạng thái từng trang:                |
| +---------------------------------------------------------------+   |
| | [Inv p1 [+]] [Inv p2 [+]] [Inv p3 [~]] [PL p1 [!]] [B/L [+]]    |   |
| |     ^                        ^            ^                   |   |
| |   OK                       Cần chú ý    Cần sửa              |   |
| +---------------------------------------------------------------+   |
|                                                                      |
| Khi nhảy tới vấn đề -> tự động đánh dấu nổi bật (pulse animation):  |
| +---------------------------------------------------------------+   |
| |                                                               |   |
| |    +========================+                                 |   |
| |    | <-<- Đơn giá: 3,000 ->->  |  <- hiệu ứng nhấp nháy         |   |
| |    +========================+                                 |   |
| |                                                               |   |
| |  Tooltip: "この項目を確認してください"                        |   |
| |           (Vui lòng xác nhận trường này)                      |   |
| +---------------------------------------------------------------+   |
+----------------------------------------------------------------------+
```

## 4.5 Màn hình Nhập Tự do (Free Input)

```
+----------------------------------------------------------------------+
| AIOCR System | RS-FB-344 | Nhập Tự do (フリー入力)                  |
+----------------------------------------------------------------------+
|                                                                      |
| Phần này cho phép nhập các mục mà AIOCR không thể xử lý tự động.   |
| Tối đa 30 mục (items).                                               |
|                                                                      |
| +---------------------------------------------------------------+   |
| | # | Tiêu đề (Title)| Thuộc tính| Chữ số| Nhập giá trị | TC  |   |
| |   |                | (Attr)    | (CS)  | (Input)      | (Th)|   |
| |---+----------------+-----------+-------+--------------+-----|   |
| | 1 | 備考           | text      |  100  | [________]   | [+]  |   |
| |   | (Ghi chú)      |           |       |              |     |   |
| |---+----------------+-----------+-------+--------------+-----|   |
| | 2 | 特殊記号        | text      |   50  | [________]   | [EDIT]  |   |
| |   | (Ký hiệu đặc   |           |       |              |     |   |
| |   |  biệt)          |           |       |              |     |   |
| |---+----------------+-----------+-------+--------------+-----|   |
| | 3 | 内容量         | number    |   10  | [500ml]      | [SYNC]  |   |
| |   | (Dung tích)    |           |       | ^ Tự động điền    |   |
| |   |                |           |       |  từ OCR: "容量"    |   |
| |---+----------------+-----------+-------+--------------+-----|   |
| |...| ...            | ...       |  ...  | ...          | ... |   |
| +---------------------------------------------------------------+   |
|                                                                      |
| Chú thích trạng thái nguồn (TC):                                    |
|   [+] = Thủ công (manual)                                            |
|   [SYNC] = Được ánh xạ từ OCR (ocr_mapped)                             |
|   [EDIT] = Thủ công, đã chỉnh sửa                                      |
|                                                                      |
| +------------------+                                                  |
| | + Thêm mục (Add) |  <-- Thêm mục mới (tối đa 30)                 |
| +------------------+                                                  |
|                                                                      |
| Định nghĩa Mục Nhập Tự do:                                          |
| +---------------------------------------------------------------+   |
| | Tiêu đề (Title):     [_________________________]              |   |
| | Thuộc tính (Attr):   [text [v]]  (text/number/date/select)     |   |
| | Chữ số (Digit):      [___]                                    |   |
| | Giá trị (Input):     [_________________________]              |   |
| |                                                                |   |
| | [SEARCH] Gợi ý ánh xạ OCR:                                         |   |
| | +-----------------------------------------------------------+ |   |
| | | Tìm thấy "容量" (Dung tích) trong kết quả OCR -> [Ánh xạ]  | |   |
| | | Tìm thấy "備考" (Ghi chú) trong kết quả OCR  -> [Ánh xạ]  | |   |
| | +-----------------------------------------------------------+ |   |
| +---------------------------------------------------------------+   |
|                                                                      |
| +------------------+ +------------------+                            |
| | Lưu (Save)       | | Quay lại (Back)  |                            |
| +------------------+ +------------------+                            |
+----------------------------------------------------------------------+
```

## 4.6 Màn hình Thứ tự Xuất -- Cấu hình Cột CSV (Output Order)

```
+----------------------------------------------------------------------+
| AIOCR System | RS-FB-344 | Thứ tự Xuất (Output Order)               |
+----------------------------------------------------------------------+
|                                                                      |
| Kéo thả để sắp xếp thứ tự cột trong file CSV output:               |
|                                                                      |
| Bước 1: Chọn dòng (mục hàng) để xuất:                               |
| +---------------------------------------------------------------+   |
| | [x] Mục #1: FLOOR CHAIR  (XY-CR-7515-C)  SL: 100            |   |
| | [ ] Mục #2: CUSHION      (XY-CS-7520-B)  SL: 200            |   |
| | [x] Mục #3: BACK REST    (XY-BR-7510-A)  SL: 50             |   |
| | [ ] Mục #4: ARMREST      (XY-AR-7525-D)  SL: 75             |   |
| | [x] Mục #5: LEG SET      (XY-LG-7530-E)  SL: 100            |   |
| |                                                               |   |
| | [Chọn tất cả] [Bỏ chọn tất cả] [Đảo ngược]                  |   |
| | Đang hiển thị 3/5 mục đã chọn                                |   |
| +---------------------------------------------------------------+   |
|                                                                      |
| Bước 2: Sắp xếp cột (Kéo thả để đổi thứ tự):                       |
| +---------------------------------------------------------------+   |
| | Cột Đang hoạt động (Kéo để sắp xếp lại):                     |   |
| |                                                                |   |
| | +-----------------------------------------------------------+ |   |
| | | :::  1. 発注番号 (Mã ĐH)                 [v Bao gồm]        | |   |
| | | :::  2. 品名 (Tên sản phẩm)               [v Bao gồm]        | |   |
| | | :::  3. 品番 (Mã sản phẩm)                [v Bao gồm]        | |   |
| | | :::  4. 数量 (Số lượng)                   [v Bao gồm]        | |   |
| | | :::  5. 単価 (Đơn giá)                    [v Bao gồm]        | |   |
| | | :::  6. 金額 (Thành tiền)                 [v Bao gồm]        | |   |
| | | :::  7. 取引先コード (Mã ĐT)              [v Bao gồm]        | |   |
| | | :::  8. 取引先名 (Tên ĐT)                 [v Bao gồm]        | |   |
| | | :::  9. 住所 (Địa chỉ)                    [v Bao gồm]        | |   |
| | | ::: 10. 出荷日 (Ngày giao)                [[ ] Loại trừ]       | |   |
| | +-----------------------------------------------------------+ |   |
| +---------------------------------------------------------------+   |
|                                                                      |
| Xem trước CSV:                                                       |
| +---------------------------------------------------------------+   |
| | Mã ĐH, Tên SP, Mã SP, SL, Đơn giá, Thành tiền, Mã ĐT, Tên ĐT|   |
| | RS-344, FLOOR CHAIR, XY-CR-7515, 100, 5000, 500000, P001,...|   |
| | RS-344, BACK REST, XY-BR-7510, 50, 8000, 400000, P001,...   |   |
| | RS-344, LEG SET, XY-LG-7530, 100, 3000, 300000, P001,...    |   |
| +---------------------------------------------------------------+   |
|                                                                      |
| +------------------------+ +------------------------+                |
| | Tải CSV (Download)     | | Quay lại (Back)        |                |
| +------------------------+ +------------------------+                |
+----------------------------------------------------------------------+
```

## 4.7 Màn hình Quản lý Đối tác (Partner Master)

```
+----------------------------------------------------------------------+
| AIOCR System | 取引先マスタ (Quản lý Đối tác)                        |
+----------------------------------------------------------------------+
|                                                                      |
| +-------------------------+ +------------------------------------+  |
| | + Thêm mới (Add)        | | [SEARCH] 検索 [________________________] |  |
| +-------------------------+ +------------------------------------+  |
|                                                                      |
| +---------------------------------------------------------------+   |
| | Mã (Code)| Tên (Name)       | Địa chỉ (Address)| Điện thoại   |   |
| |----------+------------------+-------------------+--------------|   |
| | P001     | ABC Corp         | 1-2-3 Tokyo       | 03-1234-...  |   |
| | P002     | XYZ Ltd          | 4-5-6 Osaka       | 06-5678-...  |   |
| | P003     | DEF Inc          | 7-8-9 Yokohama    | 045-901-...  |   |
| | P004     | GHI Trading      | 10-11 Nagoya      | 052-234-...  |   |
| | ...      | ...              | ...               | ...         |   |
| +---------------------------------------------------------------+   |
|                                                                      |
| +---------------------------------------------------------------+   |
| |  < 1  2  3 >                                                   |   |
| +---------------------------------------------------------------+   |
|                                                                      |
| Chi tiết / Thêm / Sửa Đối tác (Modal):                              |
| +---------------------------------------------------------------+   |
| | Mã ĐT (Code):        [P005           ]                         |   |
| | Tên ĐT (Name):       [JKL Corp       ]                         |   |
| | Địa chỉ (Address):   [12-34 Kobe     ]                         |   |
| | Điện thoại (Phone):  [078-123-4567   ]                         |   |
| |                                                                |   |
| | +------------------+ +--------------------+                    |   |
| | | Lưu (Save)       | | Hủy (Cancel)       |                    |   |
| | +------------------+ +--------------------+                    |   |
| +---------------------------------------------------------------+   |
+----------------------------------------------------------------------+
```

## 4.8 Màn hình Quản lý Mẫu Biểu mẫu (Form Template Management)

```
+----------------------------------------------------------------------+
| AIOCR System | フォーム定義管理 (Quản lý Mẫu Biểu mẫu)               |
+----------------------------------------------------------------------+
|                                                                      |
| +-------------------------+ +------------------------------------+  |
| | + Thêm mẫu mới          | | [SEARCH] Tìm kiếm [__________________] |  |
| +-------------------------+ +------------------------------------+  |
|                                                                      |
| +---------------------------------------------------------------+   |
| | Tên mẫu        | Đối tác     | Loại TL    | Tr.thái | Thao tác|   |
| |----------------+-------------+------------+---------+---------|   |
| | Generic Invoice| (Chung)     | Invoice    | [OK] Active| [EDIT] [DEL]  |   |
| | Generic PL     | (Chung)     | PL         | [OK] Active| [EDIT] [DEL]  |   |
| | Generic B/L    | (Chung)     | B/L        | [OK] Active| [EDIT] [DEL]  |   |
| | ABC Invoice    | ABC Corp    | Invoice    | [OK] Active| [EDIT] [DEL]  |   |
| | XYZ Invoice    | XYZ Ltd     | Invoice    | [OK] Active| [EDIT] [DEL]  |   |
| +---------------------------------------------------------------+   |
|                                                                      |
| Chế độ Huấn luyện (Training Mode) -- Định nghĩa vùng:                |
| +---------------------------------------------------------------+   |
| | +---------------------------+------------------------------+   |   |
| | |                           |  Định nghĩa Trường (Field): |   |   |
| | |     +=========+           |                              |   |   |
| | |     | Inv No  | <- vùng    |  Tên: [invoice_no         ] |   |   |
| | |     +=========+           |  Nhãn: [請求書番号 (Số HĐ)] |   |   |
| | |                           |  Loại: [text [v]]              |   |   |
| | |     +---+                  |  Mẫu: [RS-[A-Z]{2}-\d{3}]  |   |   |
| | |     |Dt| <- vùng           |  Bắt buộc: [v]              |   |   |
| | |     +---+                  |                              |   |   |
| | |                           |  Vùng: x=0.10 y=0.05         |   |   |
| | |      (PDF mẫu)            |        w=0.25 h=0.03         |   |   |
| | |                           |                              |   |   |
| | +---------------------------+------------------------------+   |   |
| |                                                               |   |
| | [Lưu mẫu] [Xuất JSON] [Xem trước OCR]                        |   |
| +---------------------------------------------------------------+   |
+----------------------------------------------------------------------+
```

## 4.9 Sơ đồ Điều hướng Màn hình (Screen Navigation Map)

```
+--------------+
|   Đăng nhập  |---------------------------+
|   (Login)    |                           |
+--------------+                           |
       |                                    |
       v                                    |
+--------------+     +--------------+       |
| Danh sách    |---->| Quản lý     |       |
| Đơn hàng     |<----| Đối tác     |       |
+------+-------+     +--------------+       |
       |                                     |
       +--> Tạo Đơn hàng Mới                |
       |    +--------------+                 |
       |    | Tải lên      |                 |
       |    | (Upload)     |                 |
       |    +------+-------+                 |
       |           |                         |
       |           v                         |
       |    +--------------+                 |
       |    | Xử lý AIOCR  |                 |
       |    | (Đang tải)   |                 |
       |    +------+-------+                 |
       |           |                         |
       |           v                         |
       |    +---------------------------+    |
       |    | Trình xem Tài liệu        |    |
       |    | (Split View)              |    |
       |    +--+----------+------+-----++    |
       |       |          |      |      |    |
       |       v          v      v      v    |
       |   +------+  +------+ +------+ +--+ |
       |   |Chỉnh |  |Nhập  | |Mẫu   | |XT| |
       |   |sửa   |  |Tự do | |Biểu  | |  | |
       |   |Trường|  |      | |mẫu   | |  | |
       |   +------+  +------+ +------+ +--+ |
       |                               |     |
       |                    +----------+     |
       |                    v                 |
       |             +--------------+         |
       |             | Thứ tự Xuất |         |
       |             | (Cấu hình   |         |
       |             |  CSV)       |         |
       |             +------+------+         |
       |                    |                 |
       |                    v                 |
       |             +--------------+         |
       |             | Tải CSV      |         |
       |             | (Download)   |         |
       |             +--------------+         |
       |                                       |
       +--> Chọn Đơn hàng Tồn tại             |
            (Mở Trình xem Tài liệu trực tiếp) |
                                            |  |
                                            +--+
```

## 4.10 Màn hình Tạo Đơn hàng Mới (Create Order)

```
+----------------------------------------------------------------------+
| AIOCR System | + Tạo Đơn hàng Mới (New Order)                       |
+----------------------------------------------------------------------+
|                                                                      |
| Thông tin Đơn hàng:                                                  |
| +---------------------------------------------------------------+   |
| |                                                                |   |
| |  発注番号 (Mã ĐH):        [________________]  * Bắt buộc     |   |
| |                                                                |   |
| |  取引先 (Đối tác):         [Chọn đối tác [v]]                   |   |
| |                            +-------------------------------+  |   |
| |                            | [SEARCH] Tìm kiếm mã/tên...        |  |   |
| |                            | P001 - ABC Corp              |  |   |
| |                            | P002 - XYZ Ltd               |  |   |
| |                            | P003 - DEF Inc               |  |   |
| |                            +-------------------------------+  |   |
| |                                                                |   |
| |  メモ (Ghi chú):           [________________]                |   |
| |                                                                |   |
| +---------------------------------------------------------------+   |
|                                                                      |
| Trạng thái mặc định: Đã tạo (Created)                               |
|                                                                      |
| +------------------+ +------------------+ +------------------+       |
| | Tạo & Tải lên   | | Tạo & Quay lại  | | Hủy (Cancel)     |       |
| | (Create & Upload)| | (Create & Back) | |                  |       |
| +------------------+ +------------------+ +------------------+       |
+----------------------------------------------------------------------+
```

## 4.11 Màn hình Chi tiết Đơn hàng (Order Detail)

```
+----------------------------------------------------------------------+
| AIOCR System | RS-FB-344 | Chi tiết Đơn hàng (Order Detail)         |
+----------------------------------------------------------------------+
|                                                                      |
| +---------------------------------------------------------------+   |
| | Thông tin Đơn hàng:                                           |   |
| |                                                                |   |
| | 発注番号 (Mã ĐH):     RS-FB-344                              |   |
| | 取引先 (Đối tác):      ABC Corp (P001)                        |   |
| | ステータス (Trạng thái): [Review [v]]                           |   |
| | 登録日 (Ngày tạo):     2026/03/15                             |   |
| | 更新日 (Cập nhật):     2026/03/16                             |   |
| +---------------------------------------------------------------+   |
|                                                                      |
| Danh sách Tài liệu (Documents):                                      |
| +---------------------------------------------------------------+   |
| | # | Tên file                | Loại   | Trang | OCR   | Thao |   |
| |---+-------------------------+--------+-------+-------+------|   |
| | 1 | invoice_RS-FB-344.pdf   |Invoice |  3    | [OK] OK | [DEL]   |   |
| | 2 | packinglist_RS-FB-344.pdf|PL      |  2    | [OK] OK | [DEL]   |   |
| | 3 | bl_RS-FB-344.pdf        |B/L     |  2    | [OK] OK | [DEL]   |   |
| +---------------------------------------------------------------+   |
|                                                                      |
| Kiểm tra Nhất quán (Consistency Checks):                             |
| +---------------------------------------------------------------+   |
| | Kiểm tra         | Nguồn          | Đích          | Kết quả |   |
| |------------------+----------------+---------------+---------|   |
| | Số tiền (Amount) | Invoice: 1.1M  | PL: 1.1M      | [OK] Khớp |   |
| | Số lượng (Qty)   | Inv: 100+200   | PL: 100+200   | [OK] Khớp |   |
| | Đối tác (Partner)| Inv: ABC Corp  | B/L: ABC Corp | [!] Khác |   |
| +---------------------------------------------------------------+   |
|                                                                      |
| Nhập Tự do (Free Input):    3/30 mục đã nhập                       |
| Xuất CSV (CSV Export):       Đã cấu hình                             |
|                                                                      |
| +------------------+ +------------------+ +------------------+       |
| | Xem Tài liệu    | | Chỉnh sửa ĐH    | | Quay lại         |       |
| | (View Documents) | | (Edit Order)    | | (Back)           |       |
| +------------------+ +------------------+ +------------------+       |
+----------------------------------------------------------------------+
```

## 4.12 Màn hình Tiến trình Xử lý AIOCR (Processing Status)

```
+----------------------------------------------------------------------+
| AIOCR System | RS-FB-344 | Đang xử lý AIOCR...                     |
+----------------------------------------------------------------------+
|                                                                      |
| Tiến trình xử lý hàng loạt (Batch Processing):                       |
|                                                                      |
| +---------------------------------------------------------------+   |
| |                                                                |   |
| |   Tổng thể (Overall):  [==========75%==========]    3/4       |   |
| |                                                                |   |
| |   +-------------------------------------------------------+   |   |
| |   | [OK] Invoice (Hóa đơn)      100%   Hoàn tất            |   |   |
| |   |   Trích xuất: 18 trường | [+] 15  [~] 2  [!] 1         |   |   |
| |   +-------------------------------------------------------+   |   |
| |   | [OK] PL (Danh sách đóng gói) 100%   Hoàn tất          |   |   |
| |   |   Trích xuất: 12 trường | [+] 11  [~] 1  [!] 0         |   |   |
| |   +-------------------------------------------------------+   |   |
| |   | [SYNC] B/L (Vận đơn)         45%    Đang xử lý...       |   |   |
| |   |   ========--------------                              |   |   |
| |   +-------------------------------------------------------+   |   |
| |   | [WAIT] Chờ xử lý            0%     Hàng đợi             |   |   |
| |   +-------------------------------------------------------+   |   |
| |                                                                |   |
| |   Thời gian đã qua: 45s | Dự kiến còn lại: ~30s            |   |
| |                                                                |   |
| +---------------------------------------------------------------+   |
|                                                                      |
| +------------------+                                                  |
| | Hủy xử lý (Cancel)|  <- Chỉ hiện khi đang xử lý                  |
| +------------------+                                                  |
+----------------------------------------------------------------------+
```

## 4.13 Màn hình Báo cáo Chi tiết Kiểm tra Nhất quán (Consistency Report)

```
+----------------------------------------------------------------------+
| AIOCR System | RS-FB-344 | Báo cáo Nhất quán (Consistency Report)   |
+----------------------------------------------------------------------+
|                                                                      |
| +---------------------------------------------------------------+   |
| | Kết quả kiểm tra:                    Tổng: 4 | [OK] 2 | [!] 2    |   |
| +---------------------------------------------------------------+   |
|                                                                      |
| Kiểm tra 1: Số tiền (Amount)                                        |
| +---------------------------------------------------------------+   |
| | [OK] KHỚP                                                        |   |
| | Nguồn:  Invoice (invoice_RS-FB-344.pdf) trang 1               |   |
| |   Trường: Total Amount = 1,100,000 JPY                         |   |
| | Đích:    PL (packinglist_RS-FB-344.pdf) trang 1                |   |
| |   Trường: Total Amount = 1,100,000 JPY                         |   |
| |                                                                 |   |
| | [Xem trên Document Viewer ->]                                   |   |
| +---------------------------------------------------------------+   |
|                                                                      |
| Kiểm tra 2: Số lượng (Quantity)                                     |
| +---------------------------------------------------------------+   |
| | [!] KHÔNG KHỚP                                                  |   |
| | Nguồn:  Invoice trang 1                                        |   |
| |   Mục #1: FLOOR CHAIR - SL: 100                               |   |
| |   Mục #2: CUSHION    - SL: 200                                |   |
| | Đích:    PL trang 1                                             |   |
| |   Mục #1: FLOOR CHAIR - SL: 100  [OK]                           |   |
| |   Mục #2: CUSHION    - SL: 250  [X] Khác: 200 vs 250           |   |
| |                                                                 |   |
| | [Xem trên Document Viewer ->]                                   |   |
| +---------------------------------------------------------------+   |
|                                                                      |
| Kiểm tra 3: Đối tác (Partner)                                       |
| +---------------------------------------------------------------+   |
| | [!] KHÔNG KHỚP (Khớp mờ - Fuzzy match: 85%)                   |   |
| | Nguồn:  Invoice -> Seller = "ABC Trading Co., Ltd."             |   |
| | Đích:    B/L    -> Shipper = "ABC Trading Co."                  |   |
| |   Khác: "Ltd." bị thiếu                                        |   |
| |                                                                 |   |
| | [Xem trên Document Viewer ->]                                   |   |
| +---------------------------------------------------------------+   |
|                                                                      |
| +------------------+ +------------------+                            |
| | Mở Document      | | Quay lại         |                            |
| | Viewer để sửa    | | (Back)           |                            |
| +------------------+ +------------------+                            |
+----------------------------------------------------------------------+
```

## 4.14 Màn hình Lịch sử & Theo dõi Đơn hàng (Order History)

```
+----------------------------------------------------------------------+
| AIOCR System | RS-FB-344 | Lịch sử Đơn hàng (Order History)         |
+----------------------------------------------------------------------+
|                                                                      |
| Dòng thời gian (Timeline):                                           |
| +---------------------------------------------------------------+   |
| |                                                                |   |
| |  2026/03/15 09:00  [Đã tạo - Created]                        |   |
| |       Người dùng: admin                                        |   |
| |       Mã ĐH: RS-FB-344, Đối tác: ABC Corp                    |   |
| |       |                                                        |   |
| |       v                                                        |   |
| |  2026/03/15 09:05  [Đã tải lên - Uploaded]                    |   |
| |       Tải lên 3 tài liệu (7 trang tổng)                       |   |
| |       |                                                        |   |
| |       v                                                        |   |
| |  2026/03/15 09:06  [Đang xử lý - Processing]                  |   |
| |       Bắt đầu xử lý AIOCR hàng loạt                            |   |
| |       |                                                        |   |
| |       v                                                        |   |
| |  2026/03/15 09:08  [Xem xét - Review]                         |   |
| |       OCR hoàn tất: 45 trường trích xuất                       |   |
| |       Kiểm tra nhất quán: 2/4 khớp, 2 khác biệt               |   |
| |       |                                                        |   |
| |       v                                                        |   |
| |  2026/03/15 09:15  [Đang chỉnh sửa - Editing]                  |   |
| |       admin chỉnh sửa 5 trường, gộp 2 khối                    |   |
| |                                                                |   |
| +---------------------------------------------------------------+   |
|                                                                      |
| Thống kê phiên này:                                                  |
| +---------------------------------------------------------------+   |
| | Trường đã chỉnh sửa: 5 | Khối đã gộp: 2 | Nhập tự do: 3     |   |
| +---------------------------------------------------------------+   |
|                                                                      |
| +------------------+                                                  |
| | Quay lại (Back)  |                                                  |
| +------------------+                                                  |
+----------------------------------------------------------------------+
```

## 4.15 Màn hình Lỗi Xử lý OCR (OCR Error Handling)

```
+----------------------------------------------------------------------+
| AIOCR System | RS-FB-344 | Lỗi Xử lý OCR                            |
+----------------------------------------------------------------------+
|                                                                      |
| +---------------------------------------------------------------+   |
| | [X] LỖI XỬ LÝ OCR                                               |   |
| |                                                                |   |
| | Tài liệu: bl_RS-FB-344.pdf (Vận đơn)                          |   |
| | Lỗi: External OCR API timeout sau 60 giây                     |   |
| | Mã lỗi: OCR_TIMEOUT_001                                        |   |
| |                                                                |   |
| | Chi tiết:                                                      |   |
| | - Trang 1: [OK] Đã xử lý thành công                             |   |
| | - Trang 2: [X] Timeout khi gọi API                              |   |
| |                                                                |   |
| | Tài liệu khác trong lô (batch):                                |   |
| | [OK] invoice_RS-FB-344.pdf - Hoàn tất (18 trường)               |   |
| | [OK] packinglist_RS-FB-344.pdf - Hoàn tất (12 trường)            |   |
| | [X] bl_RS-FB-344.pdf - LỖI                                      |   |
| +---------------------------------------------------------------+   |
|                                                                      |
| +------------------+ +------------------+ +------------------+       |
| | Thử lại (Retry) | | Bỏ qua (Skip)   | | Bỏ qua tất cả   |       |
| | Chỉ trang lỗi   | | Chỉ tài liệu này| | (Skip All)       |       |
| +------------------+ +------------------+ +------------------+       |
+----------------------------------------------------------------------+
```

## 4.16 Màn hình Kết quả So khớp Đối tác (Partner Matching)

```
+----------------------------------------------------------------------+
| AIOCR System | RS-FB-344 | Kết quả So khớp Đối tác                   |
+----------------------------------------------------------------------+
|                                                                      |
| OCR đã trích xuất tên đối tác: "ABC Trading Co., Ltd."               |
|                                                                      |
| Kết quả tìm kiếm trong Cơ sở dữ liệu Đối tác:                       |
| +---------------------------------------------------------------+   |
| |                                                                |   |
| | Khớp chính xác (Exact match):                                 |   |
| | +-----------------------------------------------------------+ |   |
| | | [OK] P001 | ABC Trading Co., Ltd. | 1-2-3 Tokyo | 100%      | |   |
| | |    [Chọn đối tác này ->]                                   | |   |
| | +-----------------------------------------------------------+ |   |
| |                                                                |   |
| | Khớp mờ (Fuzzy match):                                        |   |
| | +-----------------------------------------------------------+ |   |
| | | [!] P005 | ABC Trading Co.       | 5-6 Osaka   |  85%      | |   |
| | |    [Chọn] [Xem chi tiết]                                  | |   |
| | +-----------------------------------------------------------+ |   |
| | | [!] P008 | ABC Corp              | 9-10 Sapporo|  72%      | |   |
| | |    [Chọn] [Xem chi tiết]                                  | |   |
| | +-----------------------------------------------------------+ |   |
| |                                                                |   |
| +---------------------------------------------------------------+   |
|                                                                      |
| +------------------+ +------------------+ +------------------+       |
| | Tạo Đối tác mới | | Nhập tay        | | Bỏ qua (Skip)    |       |
| | (Create New)     | | (Enter Manual)  | |                  |       |
| +------------------+ +------------------+ +------------------+       |
+----------------------------------------------------------------------+
```

## 4.17 Màn hình Xem trước CSV & Tải về (CSV Preview & Download)

```
+----------------------------------------------------------------------+
| AIOCR System | RS-FB-344 | Xem trước CSV                             |
+----------------------------------------------------------------------+
|                                                                      |
| Cấu hình xuất:                                                       |
|   Dòng đã chọn: 3/5 | Cột đã chọn: 9/10 | Định dạng: UTF-8 CRLF  |
|                                                                      |
| Xem trước CSV (15 dòng đầu tiên):                                    |
| +---------------------------------------------------------------+   |
| | 発注番号, 品名, 品番, 数量, 単価, 金額, 取引先コード, ...   |   |
| | RS-344, FLOOR CHAIR, XY-CR-7515, 100, 5000, 500000, P001 |   |
| | RS-344, BACK REST, XY-BR-7510, 50, 8000, 400000, P001    |   |
| | RS-344, LEG SET, XY-LG-7530, 100, 3000, 300000, P001     |   |
| +---------------------------------------------------------------+   |
|                                                                      |
| Thông tin file:                                                      |
| +---------------------------------------------------------------+   |
| | Tên file: order_RS-FB-344_20260315.csv                       |   |
| | Kích thước: 1.2 KB                                            |   |
| | Mã hóa: UTF-8                                                  |   |
| | Kết thúc dòng: CRLF (Windows compatible)                      |   |
| | Dòng dữ liệu: 3                                               |   |
| +---------------------------------------------------------------+   |
|                                                                      |
| Trạng thái đơn hàng sẽ chuyển sang: Đã xuất (Exported)               |
|                                                                      |
| +------------------------+ +------------------+                      |
| | [SAVE] Tải CSV (Download)  | | Quay lại (Back)  |                      |
| +------------------------+ +------------------+                      |
+----------------------------------------------------------------------+
```

## 4.18 Màn hình Xác nhận Xóa (Delete Confirmation)

```
+----------------------------------------------------------------------+
|                                                                      |
|  +-----------------------------------------------------------+      |
|  |                                                            |      |
|  |  [!] XÁC NHẬN XÓA (DELETE CONFIRMATION)                    |      |
|  |                                                            |      |
|  |  Bạn có chắc chắn muốn xóa?                               |      |
|  |                                                            |      |
|  |  Đơn hàng: RS-FB-344                                      |      |
|  |  Đối tác:  ABC Corp                                       |      |
|  |  Tài liệu: 3 files (7 trang)                             |      |
|  |  Kết quả OCR: 45 trường                                   |      |
|  |                                                            |      |
|  |  [!] Hành động này không thể hoàn tác!                     |      |
|  |  (This action cannot be undone!)                           |      |
|  |                                                            |      |
|  |  +------------------+ +--------------------+               |      |
|  |  | Xóa (Delete)     | | Hủy (Cancel)       |               |      |
|  |  +------------------+ +--------------------+               |      |
|  |                                                            |      |
|  +-----------------------------------------------------------+      |
|                                                                      |
+----------------------------------------------------------------------+
```


---

# 05. Data Model (Mô hình dữ liệu)

## 5.1 Entity Relationship Diagram

```
+---------------------+       +----------------------+
|      users          |       |      orders          |
+---------------------+       +----------------------+
| PK id               |       | PK id                |
|    username         |       |    発注番号           |
|    password (hash)  |<--^   |    order_number      |
|    role             |   |   |    status             |
|    created_at       |   |   |    partner_id   -----+--+
|    updated_at       |   |   |    created_at        |  |
+---------------------+   |   |    updated_at        |  |
                          |   +----------------------+  |
                          |              |               |
                          |              | 1:N           |
                          |              v               |
                          |   +----------------------+   |
                          |   |    documents         |  |
                          |   +----------------------+   |
                          |   | PK id                |  |
                          |   | FK order_id          |  |
                          |   |    file_path         |  |
                          |   |    doc_type          |  |
                          |   |    (invoice/pl/bl)   |  |
                          |   |    page_count        |  |
                          |   |    ocr_status        |  |
                          |   |    created_at        |  |
                          |   +----------------------+   |
                          |              |               |
                          |              | 1:N           |
                          |              v               |
                          |   +----------------------+   |
                          |   |    ocr_results       |  |
                          |   +----------------------+   |
                          |   | PK id                |  |
                          |   | FK document_id       |  |
                          |   | FK order_id          |  |
                          |   |    field_name        |  |
                          |   |    field_value       |  |
                          |   |    confidence_score  |  |
                          |   |    bbox (JSON)       |  |
                          |   |    page_number       |  |
                          |   |    is_edited         |  |
                          |   |    edited_value      |  |
                          |   |    block_group_id    |  |
                          |   |    is_merged         |  |
                          |   |    merged_children   |  |
                          |   |    is_selected       |  |
                          |   |    validation_status |  |
                          |   |    validation_details|  |
                          |   |    created_at        |  |
                          |   |    updated_at        |  |
                          |   +----------------------+   |
                          |                               |
                          |              +----------------+
                          |              v
                          |   +----------------------+
                          |   |  partners            |
                          |   +----------------------+
                          |   | PK id                |
                          |   |    partner_code      |
                          |   |    partner_name      |
                          |   |    address           |
                          |   |    phone             |
                          |   |    created_at        |
                          |   |    updated_at        |
                          |   +----------------------+
                          |
                          |   +----------------------+
                          |   |  free_inputs         |
                          |   +----------------------+
                          |   | PK id                |
                          |   | FK order_id          |
                          |   |    item_number (1-30)|
                          |   |    title             |
                          |   |    attribute_type    |
                          |   |    digit_count       |
                          |   |    input_value       |
                          |   |    source_type       |
                          |   |    ocr_field_mapping |
                          |   |    ocr_result_id     |
                          |   |    created_at        |
                          |   |    updated_at        |
                          |   +----------------------+
                          |
                          |   +----------------------+
                          |   |  csv_configs         |
                          |   +----------------------+
                          |   | PK id                |
                          |   | FK order_id          |
                          |   |    column_name       |
                          |   |    sort_order        |
                          |   |    is_included       |
                          |   |    created_at        |
                          |   |    updated_at        |
                          |   +----------------------+
                          |
                          |   +----------------------+
                          |   |  consistency_checks  |
                          |   +----------------------+
                          |   | PK id                |
                          |   | FK order_id          |
                          |   |    check_type        |
                          |   |    source_doc_id     |
                          |   |    target_doc_id     |
                          |   |    source_field      |
                          |   |    target_field      |
                          |   |    source_value      |
                          |   |    target_value      |
                          |   |    is_match          |
                          |   |    created_at        |
                          |   +----------------------+
                          |
                          +--- FK: created_by (user_id)

                          +----------------------+
                          |  form_templates      |
                          +----------------------+
                          | PK id                |
                          | FK partner_id        |
                          |    doc_type          |
                          |    template_name     |
                          |    field_definitions |
                          |    (JSON)            |
                          |    is_active         |
                          |    priority          |
                          |    created_at        |
                          |    updated_at        |
                          +----------------------+
```

## 5.2 Table Definitions

### users
```sql
CREATE TABLE users (
    id BIGINT PRIMARY KEY AUTO_INCREMENT,
    username VARCHAR(100) NOT NULL UNIQUE,
    password VARCHAR(255) NOT NULL,
    role ENUM('admin', 'user') DEFAULT 'user',
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP
);
```

### orders (Cases)
```sql
CREATE TABLE orders (
    id BIGINT PRIMARY KEY AUTO_INCREMENT,
    order_number VARCHAR(50) NOT NULL UNIQUE,  -- 発注番号
    status ENUM('created','uploaded','processing','review','editing',
                'confirmed','exported') DEFAULT 'created',
    partner_id BIGINT,
    created_by BIGINT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    FOREIGN KEY (partner_id) REFERENCES partners(id),
    FOREIGN KEY (created_by) REFERENCES users(id)
);
```

### documents
```sql
CREATE TABLE documents (
    id BIGINT PRIMARY KEY AUTO_INCREMENT,
    order_id BIGINT NOT NULL,
    file_path VARCHAR(500) NOT NULL,
    file_name VARCHAR(255) NOT NULL,
    doc_type ENUM('invoice', 'packing_list', 'bill_of_lading') NOT NULL,
    page_count INT DEFAULT 0,
    ocr_status ENUM('pending','processing','completed','failed')
        DEFAULT 'pending',
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    FOREIGN KEY (order_id) REFERENCES orders(id) ON DELETE CASCADE
);
```

### ocr_results
```sql
CREATE TABLE ocr_results (
    id BIGINT PRIMARY KEY AUTO_INCREMENT,
    document_id BIGINT NOT NULL,
    order_id BIGINT NOT NULL,
    field_name VARCHAR(100) NOT NULL,
    field_value TEXT,
    confidence_score DECIMAL(5,4),  -- 0.0000 to 1.0000
    bbox JSON,                      -- Bounding box coordinates
    page_number INT DEFAULT 1,
    is_edited BOOLEAN DEFAULT FALSE,
    edited_value TEXT,
    -- Audit additions (Issue #1, #2, #3)
    block_group_id INT NULL,              -- Group ID for merged blocks
    is_merged BOOLEAN DEFAULT FALSE,      -- TRUE if this is a merged field
    merged_children JSON NULL,            -- [{id, text}] of merged sub-blocks
    is_selected BOOLEAN DEFAULT TRUE,     -- FALSE = excluded from CSV export
    validation_status ENUM('green','yellow','red') DEFAULT 'green',
    validation_details JSON NULL,         -- {"format_valid":true, "consistent":true, ...}
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    FOREIGN KEY (document_id) REFERENCES documents(id) ON DELETE CASCADE,
    FOREIGN KEY (order_id) REFERENCES orders(id) ON DELETE CASCADE
);
```

### partners
```sql
CREATE TABLE partners (
    id BIGINT PRIMARY KEY AUTO_INCREMENT,
    partner_code VARCHAR(20) NOT NULL UNIQUE,  -- 取引先コード
    partner_name VARCHAR(255) NOT NULL,         -- 取引先名
    address TEXT,                                -- 住所
    phone VARCHAR(50),                           -- 電話番号
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP
);
```

### free_inputs
```sql
CREATE TABLE free_inputs (
    id BIGINT PRIMARY KEY AUTO_INCREMENT,
    order_id BIGINT NOT NULL,
    item_number INT NOT NULL,  -- 1-30
    title VARCHAR(100),         -- タイトル
    attribute_type VARCHAR(20), -- text/number/date/select
    digit_count INT,            -- 桁数
    input_value TEXT,           -- 入力値
    -- Audit additions (Issue #4)
    source_type ENUM('manual','ocr_mapped') DEFAULT 'manual',
    ocr_field_mapping VARCHAR(100) NULL,  -- Reference to ocr_results.field_name
    ocr_result_id BIGINT NULL,            -- Direct link to auto-filled OCR result
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    FOREIGN KEY (order_id) REFERENCES orders(id) ON DELETE CASCADE,
    CONSTRAINT chk_item_number CHECK (item_number BETWEEN 1 AND 30)
);
```

### csv_configs
```sql
CREATE TABLE csv_configs (
    id BIGINT PRIMARY KEY AUTO_INCREMENT,
    order_id BIGINT NOT NULL,
    column_name VARCHAR(100) NOT NULL,
    sort_order INT NOT NULL,
    is_included BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    FOREIGN KEY (order_id) REFERENCES orders(id) ON DELETE CASCADE
);
```

### consistency_checks
```sql
CREATE TABLE consistency_checks (
    id BIGINT PRIMARY KEY AUTO_INCREMENT,
    order_id BIGINT NOT NULL,
    check_type VARCHAR(50) NOT NULL,   -- amount/quantity/partner/shipping
    source_doc_id BIGINT NOT NULL,
    target_doc_id BIGINT NOT NULL,
    source_field VARCHAR(100) NOT NULL,
    target_field VARCHAR(100) NOT NULL,
    source_value TEXT,
    target_value TEXT,
    is_match BOOLEAN,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (order_id) REFERENCES orders(id) ON DELETE CASCADE,
    FOREIGN KEY (source_doc_id) REFERENCES documents(id),
    FOREIGN KEY (target_doc_id) REFERENCES documents(id)
);
```

### form_templates (Audit Issue #5)
```sql
CREATE TABLE form_templates (
    id BIGINT PRIMARY KEY AUTO_INCREMENT,
    partner_id BIGINT NULL,              -- NULL = generic template
    doc_type ENUM('invoice','packing_list','bill_of_lading') NOT NULL,
    template_name VARCHAR(100) NOT NULL,
    field_definitions JSON NOT NULL,     -- Region definitions per field
    is_active BOOLEAN DEFAULT TRUE,
    priority INT DEFAULT 0,              -- Higher = preferred for partner
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    FOREIGN KEY (partner_id) REFERENCES partners(id)
);
```

## 5.3 Index Design

```
+---------------------------------------------------------+
|                    INDEX STRATEGY                        |
|                                                          |
|  orders:                                                 |
|    - UNIQUE INDEX idx_order_number (order_number)        |
|    - INDEX idx_status (status)                           |
|    - INDEX idx_partner (partner_id)                      |
|    - INDEX idx_created (created_at)                      |
|                                                          |
|  documents:                                              |
|    - INDEX idx_order (order_id)                          |
|    - INDEX idx_doc_type (doc_type)                       |
|    - INDEX idx_ocr_status (ocr_status)                   |
|                                                          |
|  ocr_results:                                            |
|    - INDEX idx_document (document_id)                    |
|    - INDEX idx_order (order_id)                          |
|    - INDEX idx_confidence (confidence_score)             |
|    - INDEX idx_field_name (field_name)                   |
|    - INDEX idx_block_group (block_group_id)              |
|    - INDEX idx_selected (is_selected)                    |
|    - INDEX idx_validation (validation_status)            |
|                                                          |
|  partners:                                               |
|    - UNIQUE INDEX idx_code (partner_code)                |
|    - INDEX idx_name (partner_name)                       |
|                                                          |
|  free_inputs:                                            |
|    - INDEX idx_order (order_id)                          |
|                                                          |
|  csv_configs:                                            |
|    - INDEX idx_order_sort (order_id, sort_order)         |
|                                                          |
|  consistency_checks:                                     |
|    - INDEX idx_order (order_id)                          |
|    - INDEX idx_match (is_match)                          |
|                                                          |
|  form_templates:                                         |
|    - INDEX idx_partner_doc (partner_id, doc_type)        |
|    - INDEX idx_active (is_active, priority)              |
+---------------------------------------------------------+
```


---

# 06. Functional Specification (Đặc tả chức năng)

## 6.1 Module Overview

```
+-----------------------------------------------------------+
|                 FUNCTIONAL MODULES                         |
|                                                            |
|  +------------+  +------------+  +------------+           |
|  |  F1        |  |  F2        |  |  F3        |           |
|  |  Auth      |  |  Order     |  |  Document  |           |
|  |  Module    |  |  Module    |  |  Upload    |           |
|  +------------+  +------------+  +------------+           |
|                                                            |
|  +------------+  +------------+  +------------+           |
|  |  F4        |  |  F5        |  |  F6        |           |
|  |  AIOCR     |  |  Document  |  |  Consist.  |           |
|  |  Engine    |  |  Viewer    |  |  Check     |           |
|  +------------+  +------------+  +------------+           |
|                                                            |
|  +------------+  +------------+  +------------+           |
|  |  F7        |  |  F8        |  |  F9        |           |
|  |  Free      |  |  CSV       |  |  Partner   |           |
|  |  Input     |  |  Export    |  |  Master    |           |
|  +------------+  +------------+  +------------+           |
|                                                            |
|  +------------+  +------------+                            |
|  |  F10       |  |  F11       |                            |
|  |  Form      |  |  Block     |                            |
|  |  Templates |  |  Grouping  |                            |
|  +------------+  +------------+                            |
+-----------------------------------------------------------+
```

## 6.2 F1 - Authentication (認証)

### Functional Requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| F1-01 | Login bằng username/password | Must |
| F1-02 | JWT Token authentication | Must |
| F1-03 | Session timeout management | Must |
| F1-04 | Role-based access (admin/user) | Should |

### API Endpoints

```
+----------------------------------------------------+
|  AUTH API                                           |
|                                                     |
|  POST   /api/auth/login      -> Login              |
|  POST   /api/auth/logout     -> Logout             |
|  GET    /api/auth/me         -> Current user info  |
|  POST   /api/auth/refresh    -> Refresh token      |
+----------------------------------------------------+
```

## 6.3 F2 - Order Management (注文管理)

### Functional Requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| F2-01 | Tạo order mới với 発注番号 | Must |
| F2-02 | Danh sách orders với search/filter | Must |
| F2-03 | Chi tiết order (tổng hợp documents, status) | Must |
| F2-04 | Edit/Delete order (trước khi confirmed) | Should |
| F2-05 | Phân trang (pagination) | Must |

### API Endpoints

```
+----------------------------------------------------+
|  ORDER API                                          |
|                                                     |
|  GET    /api/orders              -> List orders     |
|  POST   /api/orders              -> Create order    |
|  GET    /api/orders/{id}         -> Order detail    |
|  PUT    /api/orders/{id}         -> Update order    |
|  DELETE /api/orders/{id}         -> Delete order    |
|  GET    /api/orders/{id}/status  -> Get status      |
+----------------------------------------------------+
```

### Order Status Flow

```
  created ---> uploaded ---> processing ---> review
                                                 |
                                +---------------+
                                |
                                v
                             editing <------+
                                |            |
                                v            |
                             confirmed       |
                                |       (re-edit)
                                v            |
                             exported --------+
```

## 6.4 F3 - Document Upload (書類アップロード)

### Functional Requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| F3-01 | Drag & Drop upload PDF files | Must |
| F3-02 | Click to browse files | Must |
| F3-03 | Auto-detect document type (Invoice/PL/B/L) | Must |
| F3-04 | Validate PDF format | Must |
| F3-05 | Preview uploaded file list | Must |
| F3-06 | Delete individual uploaded file | Should |
| F3-07 | Max ~10 PDFs per order | Must |

### Upload Validation Rules

```
+---------------------------------------------------+
|  VALIDATION RULES                                  |
|                                                    |
|  File format:    PDF only (.pdf)                   |
|  Max file size:  TBD (MB per file)                 |
|  Max files:      ~10 per order                     |
|  Doc types:      Invoice, PL, B/L                  |
|                                                    |
|  Auto-detection logic:                             |
|  - Scan first page for keywords                    |
|  - "INVOICE" -> Invoice type                       |
|  - "PACKING LIST" -> PL type                       |
|  - "BILL OF LADING" / "B/L" -> B/L type            |
+---------------------------------------------------+
```

### API Endpoints

```
+--------------------------------------------------------------+
|  DOCUMENT API                                                 |
|                                                               |
|  POST   /api/orders/{id}/documents     -> Upload files       |
|  GET    /api/orders/{id}/documents     -> List documents      |
|  GET    /api/documents/{id}            -> Document detail     |
|  DELETE /api/documents/{id}            -> Delete document     |
|  GET    /api/documents/{id}/image/{pg} -> Get page image      |
+--------------------------------------------------------------+
```

## 6.5 F4 - AIOCR Engine (AIOCR読込)

### Functional Requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| F4-01 | Batch OCR processing cho tất cả PDFs trong order | Must |
| F4-02 | Gọi external AI OCR API cho từng document | Must |
| F4-03 | Parse & structure OCR results | Must |
| F4-04 | Calculate confidence scores cho mỗi field | Must |
| F4-05 | Store bounding box coordinates | Must |
| F4-06 | Handle OCR failures gracefully | Must |
| F4-07 | View từng document bằng button Next | Must |
| F4-08 | Selective field processing -- user chọn blocks nào đưa vào output | Must |
| F4-09 | Multi-signal validation (confidence + format + consistency) | Must |
| F4-10 | 3-level validation status: green/yellow/red | Must |

### OCR Processing Logic

```
+-----------------------------------------------------+
|  AIOCR BATCH PROCESS                                |
|                                                     |
|  Input: List of PDF files per order                 |
|                                                     |
|  For each PDF:                                      |
|    1. Send PDF to external AI OCR API               |
|    2. Receive raw OCR response (text + positions)   |
|    3. Parse response into structured fields         |
|    4. Determine field types:                        |
|       - Invoice: no, date, seller, buyer,           |
|         items, qty, price, amount                   |
|       - PL: no, weight, dimensions, item count      |
|       - B/L: no, shipper, consignee, vessel,        |
|         port of loading, port of discharge          |
|    5. Calculate confidence score (0.0 - 1.0)       |
|    6. Store results in ocr_results table            |
|    7. Store bounding boxes for highlight feature    |
|                                                     |
|  Output: Structured data with confidence scores     |
|                                                     |
|  +--------------------------------------------+    |
|  |  Confidence Color Thresholds:               |    |
|  |  - Score >= 0.8  -> Green (High)            |    |
|  |  - Score < 0.8   -> Red (Low - can check)   |    |
|  +--------------------------------------------+    |
+-----------------------------------------------------+
```

## 6.6 F5 - Document Viewer (Split View)

### Functional Requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| F5-01 | Split view: PDF image (left) + structured data (right) | Must |
| F5-02 | Highlight area trên image khi select field | Must |
| F5-03 | Select field trên right panel khi click image area | Must |
| F5-04 | Inline editing cho từng field | Must |
| F5-05 | Confidence color coding (Red/Green) | Must |
| F5-06 | Next/Prev button navigation giữa documents | Must |
| F5-07 | Page number display & navigation | Must |
| F5-08 | Update button to save changes | Must |
| F5-09 | Block grouping: merge/split OCR blocks thành fields | Must |
| F5-10 | 3-level validation color (green/yellow/red) | Must |
| F5-11 | Post-OCR auto-navigate to first issue field | Must |
| F5-12 | Summary banner: total fields, high-confidence count, issue count | Must |
| F5-13 | Per-document status indicators trong navigation bar | Should |

### Split View Interaction

```
+---------------------------------------------------+
|  INTERACTION MODEL                                 |
|                                                    |
|  Click field (right) ---> Highlight area (left)   |
|  Click area (left)  ---> Select field (right)     |
|  Double-click field ---> Edit mode                |
|  Update button     ---> Save all changes          |
|  Next/Prev button  ---> Navigate documents        |
|                                                    |
|  Navigation order:                                 |
|  Invoice p1 -> p2 -> ... -> PL p1 -> p2 -> B/L   |
+---------------------------------------------------+
```

## 6.7 F6 - Consistency Check (整合性チェック)

### Functional Requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| F6-01 | Kiểm tra Amount: Invoice total vs PL total | Must |
| F6-02 | Kiểm tra Quantity: Invoice qty vs PL qty | Must |
| F6-03 | Kiểm tra Partner: Invoice seller vs B/L shipper | Must |
| F6-04 | Kiểm tra Shipping: PL weight vs B/L weight | Should |
| F6-05 | Hiển thị kết quả check trong Document Viewer | Must |
| F6-06 | Highlight mismatches | Must |

### Check Rules

```
+-----------------------------------------------------------+
|  CROSS-DOCUMENT CONSISTENCY CHECK RULES                    |
|                                                            |
|  +-- Check 1: Amount -----------------------------------+ |
|  |  Source: Invoice -> Total Amount                      | |
|  |  Target: PL -> Total Amount                           | |
|  |  Rule:  Must be equal                                 | |
|  |  Match -> Green display                               | |
|  |  Mismatch -> Red display, highlight both              | |
|  +------------------------------------------------------+ |
|                                                            |
|  +-- Check 2: Quantity ----------------------------------+ |
|  |  Source: Invoice -> Item Quantities                   | |
|  |  Target: PL -> Item Quantities                        | |
|  |  Rule:  Must be equal per item                        | |
|  +------------------------------------------------------+ |
|                                                            |
|  +-- Check 3: Partner -----------------------------------+ |
|  |  Source: Invoice -> Seller Name                       | |
|  |  Target: B/L -> Shipper Name                          | |
|  |  Rule:  Fuzzy match (allow minor differences)         | |
|  +------------------------------------------------------+ |
|                                                            |
|  +-- Check 4: Shipping ----------------------------------+ |
|  |  Source: PL -> Total Weight                           | |
|  |  Target: B/L -> Gross Weight                          | |
|  |  Rule:  Must be within tolerance                      | |
|  +------------------------------------------------------+ |
+-----------------------------------------------------------+
```

## 6.8 F7 - Free Input (フリー入力)

### Functional Requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| F7-01 | Tạo free input items (max 30 per order) | Must |
| F7-02 | Mỗi item có: title, attribute type, digit count, input field | Must |
| F7-03 | Attribute types: text, number, date, select | Must |
| F7-04 | Delete individual items | Must |
| F7-05 | Save/update all free input values | Must |
| F7-06 | OCR-mapped auto-fill: search OCR results theo title keyword, suggest mapping | Must |
| F7-07 | Manual override cho auto-filled values | Must |
| F7-08 | Visual distinction giữa manual vs OCR-mapped items | Should |

### Free Input Item Structure

```
+-----------------------------------------------------+
|  FREE INPUT ITEM STRUCTURE                           |
|                                                      |
|  +--------------+----------------------------------+|
|  | Title        | Display label of field            ||
|  |              | Ex: remarks, special symbols, etc. ||
|  +--------------+----------------------------------+|
|  | Attribute    | Data type:                        ||
|  |              | text / number / date / select      ||
|  +--------------+----------------------------------+|
|  | Digit Count  | Max characters/digits             ||
|  |              | Ex: 50, 100, 10                    ||
|  +--------------+----------------------------------+|
|  | Input Field  | Value input field                 ||
|  |              | Input type by attribute type       ||
|  +--------------+----------------------------------+|
|                                                      |
|  Constraints:                                        |
|  - Maximum 30 items per order                        |
|  - Item number: 1-30 (sequential)                    |
|  - All fields configurable per order                 |
+-----------------------------------------------------+
```

## 6.9 F8 - CSV Export (CSV出力)

### Functional Requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| F8-01 | Cấu hình thứ tự cột bằng Drag & Drop | Must |
| F8-02 | Bao gồm/loại bỏ từng cột (checkbox) | Must |
| F8-03 | Preview CSV trước khi download | Must |
| F8-04 | Download CSV file (TOSS compatible format) | Must |
| F8-05 | Filename format: order_{発注番号}_{date}.csv | Must |
| F8-06 | UTF-8 encoding | Must |
| F8-07 | Row-level selection: chọn items (dòng) để export | Must |
| F8-08 | Bulk selection: Select All / Deselect All / Invert | Should |

### CSV Output Format

```
+--------------------------------------------------------------+
|  CSV FORMAT (TOSS Compatible)                                 |
|                                                               |
|  Encoding: UTF-8                                              |
|  Delimiter: Comma (,)                                        |
|  Line ending: CRLF                                            |
|                                                               |
|  Example output:                                              |
|  +----------------------------------------------------+      |
|  | header,col1,col2,col3,col4,col5,col6,col7          |      |
|  | RS-344,FLOOR CHAIR,XY-CR-7515-C,100,5000,500000,   |      |
|  | RS-344,CUSHION,XY-CS-7520-B,200,3000,600000,       |      |
|  +----------------------------------------------------+      |
|                                                               |
|  Column ordering: User-configurable via drag & drop          |
|  Column selection: User-selectable via checkboxes            |
+--------------------------------------------------------------+
```

## 6.10 F9 - Partner Master (取引先マスタ)

### Functional Requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| F9-01 | CRUD operations cho partner master | Must |
| F9-02 | Auto-retrieval từ OCR results | Must |
| F9-03 | Search partner by name/code | Must |
| F9-04 | Tự động fill partner info khi OCR detect partner | Must |
| F9-05 | Cho phép tạo partner mới từ OCR results | Should |

### Auto-Retrieval Logic

```
+-----------------------------------------------------+
|  PARTNER AUTO-RETRIEVAL LOGIC                        |
|                                                      |
|  1. OCR extracts partner name from document          |
|                     |                                |
|                     v                                |
|  2. Search partner_master by name (fuzzy match)      |
|                     |                                |
|              +------+------+                         |
|              |             |                         |
|           Found        Not Found                    |
|              |             |                         |
|              v             v                         |
|  3a. Auto-fill:    3b. Options:                     |
|      - Code            - Create new partner          |
|      - Name            - Enter manually              |
|      - Address         - Skip                        |
|      - Phone                                        |
|                                                      |
|  Matching priority:                                  |
|  1. Exact match on partner_code                      |
|  2. Exact match on partner_name                      |
|  3. Fuzzy match on partner_name (>80% similarity)    |
+-----------------------------------------------------+
```

## 6.11 F10 - Form Template Management (フォーム定義管理)

### Functional Requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| F10-01 | CRUD operations cho form templates | Must |
| F10-02 | Generic templates cho từng doc type (Invoice/PL/B/L) | Must |
| F10-03 | Per-company template override | Should |
| F10-04 | Template auto-matching: partner_id + doc_type -> correct template | Must |
| F10-05 | Template priority system khi có multiple matches | Should |

### Template Matching Logic

```
+-----------------------------------------------------+
|  TEMPLATE MATCHING PIPELINE                          |
|                                                      |
|  1. Identify partner_id from order                   |
|  2. Search: partner_id + doc_type + is_active        |
|                                                      |
|           +----------+                               |
|           |  Found?  |                               |
|           +----+-----+                               |
|           Yes   |   No                               |
|            v         v                               |
|  Use partner    Use generic template                  |
|  template       (partner_id = NULL)                  |
|                                                      |
|  3. Apply field_definitions to OCR parsing           |
|     - Region hints -> targeted OCR extraction         |
|     - Field patterns -> validation rules              |
|     - Layout info -> table detection                  |
+-----------------------------------------------------+
```

### API Endpoints

```
+----------------------------------------------------+
|  FORM TEMPLATE API                                  |
|                                                     |
|  GET    /api/form-templates          -> List        |
|  POST   /api/form-templates          -> Create      |
|  GET    /api/form-templates/{id}     -> Detail      |
|  PUT    /api/form-templates/{id}     -> Update      |
|  DELETE /api/form-templates/{id}     -> Delete      |
|  GET    /api/form-templates/match    -> Find match  |
|         ?partner_id=&doc_type=                      |
+----------------------------------------------------+
```

## 6.12 F11 - Block Grouping (ブロック統合)

### Functional Requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| F11-01 | Auto-merge adjacent OCR blocks (proximity clustering) | Must |
| F11-02 | Manual merge: select multiple blocks -> merge button | Must |
| F11-03 | Manual split: split merged field back to original blocks | Must |
| F11-04 | Visual indication of merged blocks vs single blocks | Must |
| F11-05 | Merge history stored for undo capability | Should |

### Block Interaction Model

```
+-----------------------------------------------------+
|  BLOCK GROUPING INTERACTION                          |
|                                                      |
|  Auto-merge (default):                               |
|  - Blocks within proximity threshold -> auto-merge   |
|  - Same-line blocks -> auto-merge                    |
|  - Result: ~80% of fields correctly grouped          |
|                                                      |
|  Manual merge:                                       |
|  - Click block A                                     |
|  - Shift+click block B, C, ...                       |
|  - Click "Merge" -> creates merged field             |
|  - merged_children JSON stores original blocks       |
|                                                      |
|  Manual split:                                       |
|  - Click merged field                                |
|  - Click "Split" -> restores original blocks         |
|  - Each block becomes individual field again         |
+-----------------------------------------------------+
```

### API Endpoints

```
+----------------------------------------------------+
|  BLOCK GROUPING API                                 |
|                                                     |
|  POST /api/ocr-results/merge                        |
|        {"ids": [1,2,3]} -> Merge blocks            |
|  POST /api/ocr-results/{id}/split                   |
|        -> Split merged field back to blocks         |
|  GET  /api/documents/{id}/blocks                    |
|        -> List raw blocks before merging            |
+----------------------------------------------------+
```

## 6.13 Non-Functional Requirements

```
+-----------------------------------------------------+
|  NON-FUNCTIONAL REQUIREMENTS                         |
|                                                      |
|  Performance:                                        |
|  - OCR processing: < 60s per PDF page               |
|  - Page load: < 3s                                   |
|  - CSV generation: < 5s per order                    |
|                                                      |
|  Reliability:                                        |
|  - OCR accuracy target: > 90% for printed text      |
|  - System uptime: 99% (business hours)               |
|  - Graceful error handling for OCR failures          |
|                                                      |
|  Usability:                                          |
|  - Responsive design for standard screens            |
|  - Intuitive drag & drop interactions                |
|  - Clear color-coded confidence indicators           |
|  - Japanese language UI                              |
|                                                      |
|  Security:                                           |
|  - Authentication required for all operations        |
|  - File upload validation (PDF only)                 |
|  - CSRF protection                                   |
|  - XSS prevention                                    |
|  - SQL injection prevention (Eloquent ORM)           |
|                                                      |
|  Scalability:                                        |
|  - Support 10 partner companies                      |
|  - 30 form patterns                                  |
|  - ~10 PDFs per case                                 |
|  - Concurrent users: 5-10                            |
+-----------------------------------------------------+
```


---

# 07. Technical Specification (Đặc tả kỹ thuật)

## 7.1 Tech Stack

```
+-----------------------------------------------------------+
|                    TECH STACK                            |
|                                                          |
|  +----------------------------------------------------+  |
|  |  FRONTEND                                         |  |
|  |  +--------------+  +--------------+              |  |
|  |  | Vue.js       |  | HTML/CSS/JS  |              |  |
|  |  | (SPA)        |  |              |              |  |
|  |  +--------------+  +--------------+              |  |
|  |  Libraries:                                       |  |
|  |  - Drag & Drop library (vue-draggable)           |  |
|  |  - PDF viewer (pdf.js)                           |  |
|  |  - HTTP client (axios)                           |  |
|  +----------------------------------------------------+  |
|                                                          |
|  +----------------------------------------------------+  |
|  |  BACKEND                                          |  |
|  |  +--------------+  +--------------+              |  |
|  |  | PHP          |  | Laravel      |              |  |
|  |  |              |  | Framework    |              |  |
|  |  +--------------+  +--------------+              |  |
|  |  Components:                                      |  |
|  |  - REST API Controllers                          |  |
|  |  - Services (Business Logic)                     |  |
|  |  - Repositories (Data Access)                    |  |
|  |  - Eloquent ORM                                  |  |
|  +----------------------------------------------------+  |
|                                                          |
|  +----------------------------------------------------+  |
|  |  DATABASE                                         |  |
|  |  +--------------+                                 |  |
|  |  | MySQL 8      |                                 |  |
|  |  +--------------+                                 |  |
|  +----------------------------------------------------+  |
|                                                          |
|  +----------------------------------------------------+  |
|  |  EXTERNAL                                         |  |
|  |  +--------------+                                 |  |
|  |  | AI OCR API   |  (External OCR Service)         |  |
|  |  +--------------+                                 |  |
|  +----------------------------------------------------+  |
+-----------------------------------------------------------+
```

## 7.2 Project Estimate

```
+-----------------------------------------------------------+
|                  PROJECT ESTIMATE                        |
|                                                          |
|  +----------------------------------------------------+  |
|  |  ORIGINAL ESTIMATE (Before Audit)                 |  |
|  |  ------------------------------                    |  |
|  |  開発工数 (Development):     13.5 人日            |  |
|  |  仕様変更・機能追加 (Buffer 20%):  2.7 人日       |  |
|  |  全工数 (Total):             16.2 人日            |  |
|  |  人月工数:                   0.77 人月            |  |
|  +----------------------------------------------------+  |
|                                                          |
|  +----------------------------------------------------+  |
|  |  REVISED ESTIMATE (After Audit 2026-04-07)        |  |
|  |  ------------------------------                    |  |
|  |                                                    |  |
|  |  Phase 1: Analysis & Design                        |  |
|  |  ------------------------------                     |  |
|  |  Original tasks                      2.0 人日      |  |
|  |  + Template analysis (10 companies)  +1.0          |  |
|  |  + Block interaction design          +0.5          |  |
|  |  + Validation rules specification    +0.5          |  |
|  |  Subtotal                            4.0 人日      |  |
|  |                                                    |  |
|  |  Phase 2: Implementation                           |  |
|  |  ------------------------------                     |  |
|  |  Original tasks                      8.5 人日      |  |
|  |  + Block grouping interaction        +1.5          |  |
|  |  + Selective field processing        +0.5          |  |
|  |  + Multi-signal validation           +1.0          |  |
|  |  + Free Input TITLE+VALUE mapping    +0.5          |  |
|  |  + Form template system (MVP)        +1.0          |  |
|  |  + Selective CSV export              +0.5          |  |
|  |  + Post-OCR default UX flow          +0.5          |  |
|  |  Subtotal                           14.0 人日      |  |
|  |                                                    |  |
|  |  Phase 3: Testing                                  |  |
|  |  ------------------------------                     |  |
|  |  Original test tasks                 3.0 人日      |  |
|  |  + Template testing (per company)    +2.0          |  |
|  |  + Cross-browser interaction tests   +0.5          |  |
|  |  + Validation edge cases             +0.5          |  |
|  |  Subtotal                            6.0 人日      |  |
|  |                                                    |  |
|  |  SUMMARY                                           |  |
|  |  ------------------------------                     |  |
|  |  開発工数 (Development):     24.0 人日             |  |
|  |  仕様変更・機能追加 (Buffer 20%):  4.8 人日        |  |
|  |  ----------------------------------                |  |
|  |  全工数 (Total):             28.8 人日             |  |
|  |  人月工数:                   1.37 人月             |  |
|  |                                                    |  |
|  |  Increase: +12.6 人日 (+78%)                       |  |
|  +----------------------------------------------------+  |
+-----------------------------------------------------------+
```

### Phased Delivery Plan

```
+-----------------------------------------------------------+
|                PHASED DELIVERY PLAN                      |
|                                                          |
|  Phase 1 - MVP (16 人日, ~3 weeks)                     |
|  +-- Login / Authentication                              |
|  +-- Order CRUD                                          |
|  +-- PDF Upload (Drag & Drop)                            |
|  +-- AIOCR Processing (generic templates only)          |
|  +-- Document Viewer (split view, basic highlight)      |
|  +-- Simple confidence colors (green/red)               |
|  +-- Free Input (manual only)                           |
|  +-- CSV Export (column reorder + include/exclude)      |
|  +-- Partner Master (basic CRUD)                        |
|                                                          |
|  Phase 2 - Enhanced (12.8 人日, ~2.5 weeks)            |
|  +-- Block Grouping (merge/split)                        |
|  +-- Selective Field Processing (3-level selection)     |
|  +-- Multi-signal Validation (green/yellow/red)         |
|  +-- Free Input OCR Mapping                             |
|  +-- Selective CSV Export (row selection)               |
|  +-- Post-OCR Auto-navigation                           |
|  +-- Per-company Template Support                       |
|                                                          |
|  Phase 3 - Optimization (ongoing)                       |
|  +-- Template auto-learning from corrections            |
|  +-- Historical pattern recognition                     |
|  +-- Batch processing performance tuning                |
+-----------------------------------------------------------+
```

## 7.3 Team Structure

```
+-----------------------------------------------------------+
|                   TEAM STRUCTURE                         |
|                                                          |
|  +----------------------+                               |
|  |  チームリーダ        |  1 person                     |
|  |  (Team Leader)       |                               |
|  +----------------------+                               |
|                                                          |
|  +----------------------------------------------------+  |
|  |  System Analysis + Detail Design Documents +     |  |
|  |  Database Design + Confirmation                  |  |
|  |  1 person                                        |  |
|  +----------------------------------------------------+  |
|                                                          |
|  +----------------------+                               |
|  |  開発者              |  1 person                     |
|  |  (Developer)         |                               |
|  +----------------------+                               |
|                                                          |
|  +----------------------+                               |
|  |  UIデザイナ          |  1 person                     |
|  |  (UI Designer)       |                               |
|  +----------------------+                               |
|                                                          |
|  +----------------------+                               |
|  |  テスター            |  1 person                     |
|  |  (Tester)            |                               |
|  +----------------------+                               |
+-----------------------------------------------------------+
```

## 7.4 Deliverables

```
+-----------------------------------------------------------+
|                   DELIVERABLES                           |
|                                                          |
|  +----------------------------------------------------+  |
|  |  1. 開発プログラムー式 (Source Code)             |  |
|  |     Delivery via Git repository                  |  |
|  |     [OK] Included                                  |  |
|  +----------------------------------------------------+  |
|                                                          |
|  +----------------------------------------------------+  |
|  |  2. テスト結果 (Test Results)                    |  |
|  |     [OK] Included                                  |  |
|  +----------------------------------------------------+  |
+-----------------------------------------------------------+
```

## 7.5 Environment Configuration

### Development

```
+------------------------------------------------------------+
|  DEVELOPMENT ENVIRONMENT                              |
|                                                       |
|  Backend:                                             |
|  - Language:    PHP                                   |
|  - Framework:   Laravel                               |
|  - Database:    MySQL 8                               |
|                                                       |
|  Frontend:                                            |
|  - Framework:   Vue.js                                |
|  - Build tool:  Vite / Laravel Mix                    |
|                                                       |
|  Development Tools:                                   |
|  - Version Control: Git                               |
|  - IDE: VS Code / PhpStorm                            |
|  - API Testing: Postman                               |
|  - Local Server: Apache/Nginx + PHP-FPM               |
+------------------------------------------------------------+
```

### Production

```
+------------------------------------------------------------+
|  PRODUCTION ENVIRONMENT                               |
|                                                       |
|  Backend:                                             |
|  - Language:    PHP                                   |
|  - Framework:   Laravel                               |
|  - Database:    MySQL 8                               |
|                                                       |
|  Frontend:                                            |
|  - Framework:   Vue.js                                |
|  - Deployed as: Static assets served by Laravel       |
|                                                       |
|  Server:                                              |
|  - Web Server:  Apache/Nginx                          |
|  - PHP Runtime: PHP-FPM                               |
|  - SSL:         HTTPS required                        |
+------------------------------------------------------------+
```

## 7.6 API Design Summary

```
+--------------------------------------------------------------+
|                     API ENDPOINTS SUMMARY                    |
|                                                              |
|  Module          | Method  | Endpoint                    |  |
|  ----------------+---------+-----------------------------|  |
|  Auth            | POST    | /api/auth/login             |  |
|  Auth            | POST    | /api/auth/logout            |  |
|  Auth            | GET     | /api/auth/me                |  |
|  Orders          | GET     | /api/orders                 |  |
|  Orders          | POST    | /api/orders                 |  |
|  Orders          | GET     | /api/orders/{id}            |  |
|  Orders          | PUT     | /api/orders/{id}            |  |
|  Orders          | DELETE  | /api/orders/{id}            |  |
|  Documents       | POST    | /api/orders/{id}/documents  |  |
|  Documents       | GET     | /api/orders/{id}/documents  |  |
|  Documents       | GET     | /api/documents/{id}         |  |
|  Documents       | DELETE  | /api/documents/{id}         |  |
|  Documents       | GET     | /api/documents/{id}/image/* |  |
|  OCR             | POST    | /api/orders/{id}/ocr/process|  |
|  OCR             | GET     | /api/documents/{id}/results |  |
|  OCR             | PUT     | /api/ocr-results/{id}       |  |
|  Consistency     | GET     | /api/orders/{id}/checks     |  |
|  Free Input      | GET     | /api/orders/{id}/free-inputs|  |
|  Free Input      | POST    | /api/orders/{id}/free-inputs|  |
|  Free Input      | PUT     | /api/free-inputs/{id}       |  |
|  Free Input      | DELETE  | /api/free-inputs/{id}       |  |
|  CSV Export      | GET     | /api/orders/{id}/csv-config |  |
|  CSV Export      | PUT     | /api/orders/{id}/csv-config |  |
|  CSV Export      | GET     | /api/orders/{id}/csv/export |  |
|  Partners        | GET     | /api/partners               |  |
|  Partners        | POST    | /api/partners               |  |
|  Partners        | GET     | /api/partners/{id}          |  |
|  Partners        | PUT     | /api/partners/{id}          |  |
|  Partners        | DELETE  | /api/partners/{id}          |  |
|  Partners        | GET     | /api/partners/search?q=     |  |
|  Templates       | GET     | /api/form-templates         |  |
|  Templates       | POST    | /api/form-templates         |  |
|  Templates       | GET     | /api/form-templates/{id}    |  |
|  Templates       | PUT     | /api/form-templates/{id}    |  |
|  Templates       | DELETE  | /api/form-templates/{id}    |  |
|  Templates       | GET     | /api/form-templates/match   |  |
|  Block Grouping  | POST    | /api/ocr-results/merge      |  |
|  Block Grouping  | POST    | /api/ocr-results/{id}/split |  |
|  Block Grouping  | GET     | /api/documents/{id}/blocks  |  |
+--------------------------------------------------------------+
```

## 7.7 Project Scope Summary

```
+-----------------------------------------------------------+
|                 SCOPE SUMMARY                            |
|                                                          |
|  [OK] IN SCOPE:                                            |
|  +-- Login / Authentication                              |
|  +-- Order (Case) Management                             |
|  +-- PDF Upload (Drag & Drop)                            |
|  +-- AIOCR Batch Processing (Printed text only)          |
|  +-- Document Viewer (Split View)                        |
|  +-- Confidence-based Color Coding                       |
|  +-- Cross-document Consistency Check                    |
|  +-- Free Input (up to 30 items, with OCR mapping)       |
|  +-- Output Order Configuration (Drag & Drop)            |
|  +-- CSV Export (TOSS compatible, selective rows+cols)   |
|  +-- Partner Master Management                           |
|  +-- Form Template Management (Generic + Per-company)    |
|  +-- Block Grouping (Auto-merge + Manual merge/split)    |
|  +-- Multi-signal Validation (green/yellow/red)          |
|                                                          |
|  [X] OUT OF SCOPE:                                        |
|  +-- AI OCR for Handwritten text (手書きOCR)             |
|  +-- Direct API integration with TOSS                    |
|  +-- Document types other than Invoice/PL/B/L            |
|                                                          |
|  [STATS] TARGET METRICS:                                      |
|  +-- 10 partner companies (70% trade volume)             |
|  +-- 30 form patterns                                    |
|  +-- ~10 PDFs per case                                   |
|  +-- OCR accuracy: >90% for printed text                 |
+-----------------------------------------------------------+
```
