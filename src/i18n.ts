// Vietnamese UI text. Keys are stable so an English table can be added later.
import type { Category, Stage } from "./api";

const reasons: Record<string, string> = {
  fish_full: "Cá no rồi! Hãy chờ hết 2 tiếng nghỉ trước khi cho ăn tiếp.",
  fish_adult: "Cá đã trưởng thành ở level 100, không nhận thêm EXP.",
  fish_not_adult: "Chỉ gọi thuyền bán cá đã trưởng thành ở level 100.",
  duplicate: "File trùng nội dung đã ghi nhận — không chuyển file, không cộng EXP.",
  hunt_waiting: "Chưa có sò mới; bạn có thể quay lại sau.",
  hunt_cap: "Đã nhặt đủ CBCoin hôm nay. Sò chưa nhặt được giữ lại.",
  hunt_meeting: "Tắt Chế độ họp để gọi thuyền.",
  hunt_stale: "Lượt thuyền đã kết thúc; hãy gọi thuyền lại.",
  hunt_busy: "Móc đang hoạt động; chờ kéo về thuyền.",
  autostart_failed: "Không thay đổi được tùy chọn mở cùng hệ thống.",
  autostart_dev: "Chỉ bật mở cùng hệ thống từ bản game đã cài, không dùng bản dev.",
  dock_failed: "Không mở được nút nhanh desktop.",
  ok: "Sẵn sàng cho cá ăn",
  not_absolute: "Đường dẫn không đầy đủ",
  device_path: "Đường dẫn mạng hoặc thiết bị không được hỗ trợ",
  alternate_stream: "Luồng dữ liệu phụ (ADS) không được phép",
  parent_ref: "Đường dẫn chứa “..”",
  not_found: "Không tìm thấy",
  directory: "Chỉ nhận file, không nhận thư mục",
  not_regular: "Không phải file thường",
  link: "Là liên kết (symlink/junction) hoặc nằm trong thư mục liên kết",
  drive_root: "File nằm ngay gốc ổ đĩa",
  app_data: "Dữ liệu của chính TrashQuarium",
  blocked_location: "Thư mục hệ thống hoặc dữ liệu ứng dụng",
  package: "Nằm trong gói ứng dụng/thư viện (ví dụ Photos Library)",
  repository: "Nằm trong kho mã nguồn (.git/.svn/.hg)",
  hidden: "File ẩn",
  system: "File hệ thống",
  offline: "File chỉ có trên đám mây, chưa tải về máy",
  extension: "Định dạng này chưa được cho phép",
  installer_outside_downloads: "Bộ cài chỉ được nhận trong thư mục Downloads",
  too_large: "Lớn hơn giới hạn 4 GiB",
  cross_volume: "Khác ổ đĩa với Bụng cá — không chuyển để tránh sao chép rồi xóa",
  locked: "Đang được chương trình khác mở",
  unreadable: "Không kiểm tra được nên từ chối để an toàn",
  changed_since_preview: "File đã đổi sau khi xem trước — hãy xem lại",
  move_failed: "Không chuyển được; file vẫn ở chỗ cũ",
  journal_write: "Không ghi được sổ giao dịch; file vẫn ở chỗ cũ",
  belly_write: "Không ghi được vào Bụng cá; file vẫn ở chỗ cũ",
  index_write: "Không ghi được danh sách Bụng cá",
  simulated_crash: "Gián đoạn giữa chừng",
  busy: "Đang xử lý một thao tác khác, thử lại sau giây lát",
  not_enough_shells: "Chưa đủ CBCoin",
  tank_full: "Bể đã đầy",
  breed_same_fish: "Hãy chọn hai con cá khác nhau để ghép cặp.",
  breed_species_mismatch: "Chỉ ghép cặp được hai cá cùng loài.",
  breed_exhausted: "Giá trị cá đã về mức giá mua gốc, không sinh sản thêm được.",
  breed_eggs_invalid: "Số trứng không hợp lệ hoặc vượt mức tối đa cho phép.",
  price_changed: "Giá đã thay đổi — hãy xem lại trước khi đổi",
  unknown_species: "Loài này không có trong cửa hàng",
  save_failed: "Không lưu được tiến trình",
  save_future: "Save được tạo bởi phiên bản mới hơn. Game mở ở chế độ chỉ xem và không ghi đè.",
  save_corrupt: "Không đọc được save và mọi bản sao lưu. Dữ liệu được giữ nguyên, không bị reset.",
  index_corrupt: "Danh sách Bụng cá bị hỏng. Không file nào bị di chuyển; dữ liệu được giữ nguyên.",
  index_future: "Bụng cá được tạo bởi phiên bản mới hơn; tạm khóa để an toàn.",
  already_running: "TrashQuarium khác đang dùng hồ sơ này.",
  data_dir: "Không dùng được thư mục dữ liệu của game.",
  original_folder_missing: "Thư mục gốc không còn; file vẫn được giữ trong Bụng cá",
  original_unsafe: "Vị trí gốc không còn an toàn; file vẫn được giữ trong Bụng cá",
  stored_missing: "Không thấy file trong Bụng cá",
  name_exhausted: "Không tìm được tên trống để nhả file",
  fish_not_found: "Không tìm thấy cá được chọn",
  too_many_files: "Quá nhiều file trong một lượt",
  tank_failed: "Không mở được bể desktop",
  desktop_not_found: "Không tìm thấy màn hình desktop của Windows",
  wallpaper_layer_unavailable: "Windows chưa cung cấp lớp hình nền. Thử lại sau.",
  attach_failed: "Không gắn được bể cá vào desktop",
  unsupported_platform: "Hệ điều hành này chưa hỗ trợ bể desktop",
  sample_failed: "Không tạo được file mẫu",
  unknown: "Lỗi không xác định",
};

export function reason(code: string): string {
  if (code.startsWith("original_unsafe")) return reasons.original_unsafe;
  return reasons[code] ?? code;
}

export const attentionNote: Record<string, string> = {
  both_exist: "File có mặt ở cả hai nơi. Cả hai được giữ nguyên — hãy tự kiểm tra rồi quyết định.",
  missing: "Không thấy file ở cả hai nơi. Không có gì bị xóa bởi game; hãy kiểm tra thủ công.",
  index_mismatch: "Sổ ghi không khớp với ổ đĩa. Mọi file được giữ nguyên.",
  journal_unreadable: "Không đọc được một sổ giao dịch. Mọi file được giữ nguyên.",
};

export const stageName: Record<Stage, string> = {
  fry: "Cá non",
  juvenile: "Cá thành niên",
  adult: "Trưởng thành",
};

export const categoryName: Record<Category, string> = {
  doc: "Tài liệu",
  media: "Ảnh/âm thanh/video",
  tech: "Nén/log/bộ cài",
};

export function formatSize(bytes: number): string {
  const units = ["B", "KB", "MB", "GB"];
  let n = bytes;
  let i = 0;
  while (n >= 1024 && i < units.length - 1) {
    n /= 1024;
    i++;
  }
  return `${n.toLocaleString("vi-VN", { maximumFractionDigits: i === 0 ? 0 : 1 })} ${units[i]}`;
}

export function formatDate(unix: number): string {
  return new Date(unix * 1000).toLocaleString("vi-VN", { dateStyle: "short", timeStyle: "short" });
}
