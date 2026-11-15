const en = {
  clickMenu: {
    label: 'Menu options',
    submit: 'Submit',
    submitTitle: 'Go to Submit and confirm',
    otherTitle: '{label} (moves the highlight here, then type in the terminal)',
    setting: 'Click bar for agent menus',
    settingHint: 'When Claude or Codex shows a choice menu, a bar under the pane lists the options; click one to send its key.',
    toast: 'Answer buttons on the notification',
    toastHint: 'When Orrery is not focused and an agent shows a menu with up to 3 options, the system notification gets one button per option. Needs notifications on.',
  },
  menuToast: {
    open: 'Open',
    dismiss: 'Dismiss',
  },
  menuQueue: {
    chip: 'Waiting for answer {n}',
    chipTip: 'Panes showing a choice menu, oldest first ({keys} jumps to the next one)',
    title: 'Waiting for answer',
    waited: 'Time waiting',
    answerLabel: 'Answer: {question}',
    more: '+{n} more',
    kind: {
      ask: 'Question',
      'ask-multi': 'Multiple choice question',
      permission: 'Permission request',
      plan: 'Plan approval',
      picker: 'Choice',
      'codex-approval': 'Codex approval',
    },
  },
}

const vi: typeof en = {
  clickMenu: {
    label: 'Đáp án menu',
    submit: 'Submit',
    submitTitle: 'Đi tới Submit rồi xác nhận',
    otherTitle: '{label} (chuyển highlight tới đây, rồi gõ trong terminal)',
    setting: 'Thanh nút cho menu của agent',
    settingHint: 'Khi Claude hoặc Codex hiện menu chọn, dưới pane có thanh liệt kê đáp án; bấm một nút để gửi phím tương ứng.',
    toast: 'Toast có nút đáp án',
    toastHint: 'Khi Orrery không focus và agent hiện menu có tối đa 3 đáp án, toast hệ thống có một nút cho mỗi đáp án. Cần bật thông báo.',
  },
  menuToast: {
    open: 'Mở',
    dismiss: 'Bỏ qua',
  },
  menuQueue: {
    chip: 'Chờ trả lời {n}',
    chipTip: 'Các pane đang hiện menu chọn, cũ nhất trước ({keys} nhảy tới pane kế tiếp)',
    title: 'Chờ trả lời',
    waited: 'Thời gian đã chờ',
    answerLabel: 'Trả lời: {question}',
    more: '+{n} nữa',
    kind: {
      ask: 'Câu hỏi',
      'ask-multi': 'Câu hỏi chọn nhiều',
      permission: 'Xin quyền',
      plan: 'Duyệt plan',
      picker: 'Lựa chọn',
      'codex-approval': 'Codex xin duyệt',
    },
  },
}

export default { en, vi }
