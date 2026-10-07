<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { use } from 'echarts/core'
import { LineChart, PieChart } from 'echarts/charts'
import { GridComponent, TooltipComponent } from 'echarts/components'
import { CanvasRenderer } from 'echarts/renderers'
import VChart from 'vue-echarts'
import type { CountTrendResponse, RiskStatsResponse, StringCount } from '@serverrs/sdk'
import { Music2, ShieldAlert, Users, ArrowUpRight } from '@lucide/vue'
import { RouterLink } from 'vue-router'
import { api } from '@/lib/sdk'
import { formatNumber, errorMessage } from '@/utils/format'

// 按需注册 ECharts 组件
use([LineChart, PieChart, GridComponent, TooltipComponent, CanvasRenderer])

const loading = ref(true)
const error = ref('')
const totals = ref({ users: 0, risks: 0, music: 0 })

const usersStats = ref<CountTrendResponse | null>(null)
const risksStats = ref<RiskStatsResponse | null>(null)
const musicStats = ref<CountTrendResponse | null>(null)

/* ── ECharts 配置 ── */

const sparklineOption = (data: number[]) => ({
  grid: { show: false, left: 0, right: 0, top: 4, bottom: 0 },
  xAxis: { show: false },
  yAxis: { show: false },
  series: [{
    type: 'line' as const, data, smooth: true,
    lineStyle: { width: 2, color: '#156354' },
    areaStyle: { color: 'rgba(21, 99, 84, 0.08)' },
    showSymbol: false,
  }],
})

const trendOption = (data: number[], labels: string[]) => ({
  tooltip: { trigger: 'axis' as const },
  grid: { left: 40, right: 16, top: 24, bottom: 24 },
  xAxis: { type: 'category' as const, data: labels, axisLabel: { fontSize: 11, color: '#63756f' } },
  yAxis: { type: 'value' as const, min: 0, splitLine: { lineStyle: { color: 'rgba(190,205,200,0.3)' } } },
  series: [{
    type: 'line' as const, data, smooth: true,
    lineStyle: { width: 2.5, color: '#156354' },
    areaStyle: { color: 'rgba(21, 99, 84, 0.06)' },
    showSymbol: true, symbol: 'circle', symbolSize: 6,
    itemStyle: { color: '#156354' },
  }],
})

const riskLevelColors: Record<string, string> = {
  none: '#8ba098',
  low: '#d8913a',
  medium: '#e8b45e',
  high: '#b94242',
  crisis: '#8b1a1a',
}

function pieOption(distribution: StringCount[]) {
  const data = distribution.map((item) => ({
    value: item.count,
    name: item.label,
    itemStyle: { color: riskLevelColors[item.label] ?? '#63756f' },
  }))
  return {
    tooltip: { trigger: 'item' as const, formatter: '{b}: {c} ({d}%)' },
    series: [{
      type: 'pie' as const, radius: ['42%', '68%'],
      data: data.length ? data : [{ value: 0, name: 'none', itemStyle: { color: '#8ba098' } }],
      label: { show: true, color: '#63756f', fontSize: 11, formatter: '{b}: {d}%' },
      emphasis: { scale: false },
      labelLine: { lineStyle: { color: 'rgba(190,205,200,0.5)' } },
    }],
  }
}

async function load() {
  loading.value = true
  error.value = ''

  const [users, risks, music, us, rs, ms] = await Promise.allSettled([
    api.admin.users({ page: 1, pageSize: 1 }),
    api.admin.riskConversations({ page: 1, pageSize: 1 }),
    api.admin.tracks({ page: 1, pageSize: 1 }),
    api.admin.statsUsers(),
    api.admin.statsRisks(),
    api.admin.statsMusic(),
  ])

  if (users.status === 'fulfilled') totals.value.users = users.value.total
  if (risks.status === 'fulfilled') totals.value.risks = risks.value.total
  if (music.status === 'fulfilled') totals.value.music = music.value.total

  if (us.status === 'fulfilled') usersStats.value = us.value
  if (rs.status === 'fulfilled') risksStats.value = rs.value
  if (ms.status === 'fulfilled') musicStats.value = ms.value

  const rejected = [users, risks, music, us, rs, ms].find((item) => item.status === 'rejected')
  if (rejected?.status === 'rejected') error.value = errorMessage(rejected.reason)
  loading.value = false
}

onMounted(load)
</script>

<template>
  <div>
    <header class="page-heading">
      <div>
        <h1>运行概览</h1>
        <p>关键业务数据与待处理事项的实时入口。</p>
      </div>
      <button class="button" :disabled="loading" @click="load">刷新数据</button>
    </header>

    <p v-if="error" class="notice">{{ error }}，部分数据可能未加载。</p>

    <!-- 统计卡 + 迷你趋势图 -->
    <section class="stats-grid">
      <article class="card stat-card">
        <div class="stat-icon"><Users :size="19" /></div>
        <strong>{{ loading ? '—' : formatNumber(totals.users) }}</strong>
        <span>系统用户总数</span>
        <VChart v-if="!loading" class="sparkline" :option="sparklineOption(usersStats?.trend.map(t => t.count) ?? [])" autoresize />
      </article>
      <article class="card stat-card">
        <div class="stat-icon"><ShieldAlert :size="19" /></div>
        <strong>{{ loading ? '—' : formatNumber(totals.risks) }}</strong>
        <span>已审计会话</span>
        <VChart v-if="!loading" class="sparkline" :option="sparklineOption(risksStats?.trend.map(t => t.count) ?? [])" autoresize />
      </article>
      <article class="card stat-card">
        <div class="stat-icon"><Music2 :size="19" /></div>
        <strong>{{ loading ? '—' : formatNumber(totals.music) }}</strong>
        <span>音乐资源总数</span>
        <VChart v-if="!loading" class="sparkline" :option="sparklineOption(musicStats?.trend.map(t => t.count) ?? [])" autoresize />
      </article>
    </section>

    <!-- 图表区域 -->
    <section class="chart-grid">
      <article class="card">
        <div class="card-header"><h2>风险等级分布</h2></div>
        <div class="card-body chart-body">
          <VChart v-if="!loading" class="chart" :option="pieOption(risksStats?.distribution ?? [])" autoresize />
        </div>
      </article>
      <article class="card">
        <div class="card-header"><h2>近 7 天风险趋势</h2></div>
        <div class="card-body chart-body">
          <VChart v-if="!loading" class="chart" :option="trendOption(
            risksStats?.trend.map(t => t.count) ?? [],
            risksStats?.trend.map(t => t.label) ?? [],
          )" autoresize />
        </div>
      </article>
    </section>

    <section class="content-grid">
      <article class="card quick-card">
        <div class="card-header"><h2>快速入口</h2></div>
        <div class="card-body quick-links">
          <RouterLink to="/risks">
            <ShieldAlert :size="20" />
            <div><strong>查看风险</strong><span>查看高风险对话内容</span></div>
          </RouterLink>
          <RouterLink to="/users">
            <Users :size="20" />
            <div><strong>管理用户</strong><span>调整账号状态和角色</span></div>
          </RouterLink>
        </div>
      </article>
    </section>
  </div>
</template>

<style scoped>
.notice {
  margin-bottom: 18px;
}

.sparkline {
  height: 48px;
  margin-top: 8px;
}

.chart-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 16px;
  margin-bottom: 20px;
}

.chart-body {
  padding: 8px;
}

.chart {
  height: 260px;
  width: 100%;
}

.stats-grid {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 16px;
  margin-bottom: 20px;
}

.stat-card {
  position: relative;
  min-height: 138px;
  overflow: hidden;
  padding: 20px;
}

.stat-card::after {
  position: absolute;
  right: -26px;
  bottom: -42px;
  width: 110px;
  height: 110px;
  border-radius: 50%;
  background: var(--brand-soft);
  content: "";
}

.stat-icon {
  display: grid;
  width: 38px;
  height: 38px;
  place-items: center;
  border-radius: 11px;
  color: var(--brand);
  background: var(--brand-soft);
}

.stat-card strong {
  display: block;
  margin-top: 14px;
  font-family: Georgia, serif;
  font-size: 28px;
  line-height: 1;
}

.stat-card span {
  display: block;
  margin-top: 7px;
  color: var(--muted);
  font-size: 12px;
}

.content-grid {
  display: grid;
  grid-template-columns: 1fr;
  gap: 20px;
}

.quick-links {
  display: grid;
  gap: 10px;
}

.quick-links a {
  display: flex;
  align-items: center;
  gap: 13px;
  padding: 13px;
  border: 1px solid var(--line);
  border-radius: 11px;
}

.quick-links a:hover {
  border-color: #aec1ba;
  background: #f8fbfa;
}

.quick-links svg {
  color: var(--brand);
}

.quick-links div {
  display: flex;
  flex-direction: column;
}

.quick-links span {
  color: var(--muted);
  font-size: 11px;
}

@media (max-width: 1100px) {
  .stats-grid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
  .chart-grid {
    grid-template-columns: 1fr;
  }
  .content-grid {
    grid-template-columns: 1fr;
  }
}

@media (max-width: 760px) {
  .stats-grid {
    grid-template-columns: 1fr;
  }
}
</style>
