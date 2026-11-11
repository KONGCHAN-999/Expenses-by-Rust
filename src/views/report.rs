use expenses_core::date::YearMonth;
use expenses_core::format::{format_kip, format_number, month_name, month_year_label};
use expenses_core::model::Kind;
use expenses_core::summary::{Period, breakdown, monthly_series, top_n, totals};
use leptos::prelude::*;

use crate::components::{CatIcon, Empty, PeriodNav};
use crate::state::use_app;

const DONUT_SLICES: usize = 4;
const OTHER_COLOR: &str = "#94a3b8";
const BAR_HEIGHT_PX: i64 = 96;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Month,
    Year,
}

#[derive(PartialEq)]
struct Slice {
    label: String,
    icon: Option<(String, String)>,
    color: String,
    amount: i64,
    percent: u32,
}

#[component]
pub fn ReportView() -> impl IntoView {
    let state = use_app();
    let mode = RwSignal::new(Mode::Month);
    let kind = RwSignal::new(Kind::Expense);

    let period = Signal::derive(move || match mode.get() {
        Mode::Month => Period::Month(state.month.get()),
        Mode::Year => Period::Year(state.month.get().year),
    });
    let label = Signal::derive(move || match mode.get() {
        Mode::Month => month_year_label(state.month.get()),
        Mode::Year => format!("ປີ {}", state.month.get().year),
    });
    let step = move |forward: bool| {
        state.month.update(|m| {
            *m = match (mode.get_untracked(), forward) {
                (Mode::Month, true) => m.next(),
                (Mode::Month, false) => m.prev(),
                (Mode::Year, true) => YearMonth { year: m.year + 1, ..*m },
                (Mode::Year, false) => YearMonth { year: m.year - 1, ..*m },
            }
        })
    };

    let period_totals = Memo::new(move |_| state.data.with(|d| totals(&d.transactions, period.get())));
    let slices = Memo::new(move |_| {
        state.data.with(|d| {
            let (top, other) = top_n(breakdown(&d.transactions, kind.get(), period.get()), DONUT_SLICES);
            let mut slices: Vec<Slice> = top
                .into_iter()
                .map(|s| {
                    let c = d.category(&s.category_id);
                    Slice {
                        label: c.map_or("?".into(), |c| c.name.clone()),
                        icon: c.map(|c| (c.icon.clone(), c.color.clone())),
                        color: c.map_or(OTHER_COLOR.into(), |c| c.color.clone()),
                        amount: s.amount,
                        percent: s.percent,
                    }
                })
                .collect();
            if let Some(o) = other {
                slices.push(Slice {
                    label: format!("ອື່ນໆ ({} ໝວດ)", o.count),
                    icon: None,
                    color: OTHER_COLOR.into(),
                    amount: o.amount,
                    percent: o.percent,
                });
            }
            slices
        })
    });
    let donut_style = move || {
        slices.with(|slices| {
            let total: i64 = slices.iter().map(|s| s.amount).sum();
            if total == 0 {
                return String::new();
            }
            let mut acc = 0i64;
            let stops: Vec<String> = slices
                .iter()
                .map(|s| {
                    let from = acc as f64 * 100.0 / total as f64;
                    acc += s.amount;
                    let to = acc as f64 * 100.0 / total as f64;
                    format!("{} {from:.3}% {to:.3}%", s.color)
                })
                .collect();
            format!("background: conic-gradient({})", stops.join(", "))
        })
    };
    let kind_total = move || {
        let t = period_totals.get();
        match kind.get() {
            Kind::Expense => t.expense,
            Kind::Income => t.income,
        }
    };
    let kind_word = move || match kind.get() {
        Kind::Expense => "ລາຍຈ່າຍ",
        Kind::Income => "ລາຍຮັບ",
    };

    let series = Memo::new(move |_| {
        let ym = state.month.get();
        state.data.with(|d| match mode.get() {
            Mode::Month => monthly_series(&d.transactions, ym, 6),
            Mode::Year => monthly_series(&d.transactions, YearMonth { year: ym.year, month: 12 }, 12),
        })
    });
    let bars = move || {
        let series = series.get();
        let selected = state.month.get();
        let year_mode = mode.get() == Mode::Year;
        let max = series.iter().map(|(_, t)| t.income.max(t.expense)).max().unwrap_or(0).max(1);
        let px = |v: i64| (v * BAR_HEIGHT_PX / max).max(if v > 0 { 2 } else { 0 });
        series
            .into_iter()
            .map(|(ym, t)| {
                let label = if year_mode { ym.month.to_string() } else { month_name(ym.month).to_string() };
                let title = format!("{}: ຮັບ {} · ຈ່າຍ {}", month_year_label(ym), format_number(t.income), format_number(t.expense));
                view! {
                    <button
                        class="bar"
                        class:on=selected == ym
                        title=title
                        on:click=move |_| {
                            state.month.set(ym);
                            mode.set(Mode::Month);
                        }
                    >
                        <span class="bar-pair">
                            <i class="bar-in" style=format!("height:{}px", px(t.income))></i>
                            <i class="bar-out" style=format!("height:{}px", px(t.expense))></i>
                        </span>
                        <span class="bar-label">{label}</span>
                    </button>
                }
            })
            .collect_view()
    };

    view! {
        <div class="page">
            <PeriodNav label=label on_prev=Callback::new(move |_| step(false)) on_next=Callback::new(move |_| step(true))/>
            <div class="segmented">
                <button class:on=move || mode.get() == Mode::Month on:click=move |_| mode.set(Mode::Month)>"ລາຍເດືອນ"</button>
                <button class:on=move || mode.get() == Mode::Year on:click=move |_| mode.set(Mode::Year)>"ລາຍປີ"</button>
            </div>

            <section class="card totals-strip">
                <div>
                    <small>"ລາຍຮັບ"</small>
                    <b class="pos">{move || format_kip(period_totals.get().income)}</b>
                </div>
                <div>
                    <small>"ລາຍຈ່າຍ"</small>
                    <b class="neg">{move || format_kip(period_totals.get().expense)}</b>
                </div>
                <div>
                    <small>"ຄົງເຫຼືອ"</small>
                    <b>{move || format_kip(period_totals.get().balance())}</b>
                </div>
            </section>

            <section class="card">
                <div class="card-head">
                    <span>{move || format!("{}ແຍກຕາມໝວດ", kind_word())}</span>
                    <div class="mini-toggle">
                        <button class:on=move || kind.get() == Kind::Expense on:click=move |_| kind.set(Kind::Expense)>"ຈ່າຍ"</button>
                        <button class:on=move || kind.get() == Kind::Income on:click=move |_| kind.set(Kind::Income)>"ຮັບ"</button>
                    </div>
                </div>
                <Show
                    when=move || !slices.with(Vec::is_empty)
                    fallback=move || view! { <Empty icon="📊" title="ບໍ່ມີຂໍ້ມູນໃນຊ່ວງນີ້"/> }
                >
                    <div class="donut" style=donut_style>
                        <div class="donut-hole">
                            <small>{kind_word}</small>
                            <b>{move || format_kip(kind_total())}</b>
                        </div>
                    </div>
                    <div class="legend">
                        {move || {
                            slices
                                .with(|slices| {
                                    slices
                                        .iter()
                                        .map(|s| {
                                            let icon = match &s.icon {
                                                Some((i, c)) => view! { <CatIcon icon=i.clone() color=c.clone()/> }.into_any(),
                                                None => view! { <span class="cat-icon" style=crate::components::icon_background(OTHER_COLOR)>"📦"</span> }.into_any(),
                                            };
                                            view! {
                                                <div class="legend-row">
                                                    <i class="legend-dot" style=format!("background:{}", s.color)></i>
                                                    {icon}
                                                    <span class="legend-name">{s.label.clone()}</span>
                                                    <span class="legend-value">
                                                        {format_number(s.amount)}
                                                        <small>{format!(" · {}%", s.percent)}</small>
                                                    </span>
                                                </div>
                                            }
                                        })
                                        .collect_view()
                                })
                        }}
                    </div>
                </Show>
            </section>

            <section class="card">
                <div class="card-head">
                    <span>{move || if mode.get() == Mode::Year { "ແຕ່ລະເດືອນ" } else { "6 ເດືອນຫຼ້າສຸດ" }}</span>
                    <span class="bar-legend">
                        <i class="bar-in"></i>"ຮັບ"
                        <i class="bar-out"></i>"ຈ່າຍ"
                    </span>
                </div>
                <div class="bars">{bars}</div>
                <p class="hint">"ແຕະແທ່ງເພື່ອເບິ່ງເດືອນນັ້ນ"</p>
            </section>
        </div>
    }
}
