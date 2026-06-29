export const formatCurrency = (value: number) =>
  new Intl.NumberFormat('en-US', {
    style: 'currency',
    currency: 'USD',
    maximumFractionDigits: 2
  }).format(value)

export const formatNumber = (value: number) =>
  new Intl.NumberFormat('en-US', {
    maximumFractionDigits: 4
  }).format(value)

export const formatPercent = (value: number) =>
  new Intl.NumberFormat('en-US', {
    style: 'percent',
    maximumFractionDigits: 1
  }).format(value)

export const formatDate = (value: string) =>
  new Intl.DateTimeFormat('en-US', {
    month: 'short',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit'
  }).format(new Date(value))

export const shortId = (value: string) => {
  if (value.length <= 16) {
    return value
  }
  return `${value.slice(0, 8)}...${value.slice(-6)}`
}
