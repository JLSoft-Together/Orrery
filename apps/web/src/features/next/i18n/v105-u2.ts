const en = {
  v105: {
    quota: {
      open: 'Show plan limit details for {name}',
      title: 'Plan limits',
      level: { ok: 'OK', warn: 'Running low', critical: 'Near the limit' },
      critical: 'Near limit',
      resetIn: 'resets in {rel}, {abs}',
      resetUnknown: 'reset time unknown',
      stale: 'Last report is older than 30 minutes, values may be out of date',
      unitH: 'h',
      unitMin: 'min',
      unitD: 'd',
      lessThanMin: 'under 1 min',
      forecast: 'Runs out around {at}',
      forecastShort: 'May run out before reset',
    },
  },
}

const vi = {
  v105: {
    quota: {
      open: 'Xem chi tiết hạn mức gói của {name}',
      title: 'Hạn mức gói',
      level: { ok: 'Ổn', warn: 'Gần hết', critical: 'Sắp chạm limit' },
      critical: 'Sắp hết',
      resetIn: 'reset sau {rel}, {abs}',
      resetUnknown: 'chưa biết giờ reset',
      stale: 'Báo cáo cuối đã quá 30 phút, số có thể đã cũ',
      unitH: 'giờ',
      unitMin: 'phút',
      unitD: 'ngày',
      lessThanMin: 'dưới 1 phút',
      forecast: 'Hết khoảng {at}',
      forecastShort: 'Có thể hết trước giờ reset',
    },
  },
}

export default { en, vi }
