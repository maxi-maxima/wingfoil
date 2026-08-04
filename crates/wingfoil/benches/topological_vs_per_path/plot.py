# Renders the charts for the topological-sort vs per-path-propagation
# branch/recombine comparison: cross_library.png on a linear axis,
# cross_library_log.png on a log one (same data drawn twice — linear for the
# shape, log to read the low end), plus per_cycle.png.
#
# The arrays below are *readings*, not source: refill them from a local run
# before regenerating the plots, since criterion wall-clock numbers are
# hardware-specific.
#
#   cargo bench -p wingfoil --bench bfs_vs_dfs_wingfoil
#   cargo bench -p wingfoil --bench bfs_vs_dfs_reactive
#   cargo bench -p wingfoil --features async --bench bfs_vs_dfs_async_streams
#   python plot.py
#
# Read the numbers off the criterion *console output* of each run, not out of
# `target/criterion/`: the reactive and async targets both name their
# benchmarks `depth_1`..`depth_10`, so whichever ran last owns those
# directories on disk. (The wingfoil target names its groups `cycles_depth_N`
# precisely to stay out of that collision.)
#
# The wingfoil target emits three series, one per engine tier, from a single set
# of `nitro!` blocks: `cycles_depth_N/{interpreted,compiled,nested}`, each a
# fixed 10 000-cycle run of a self-contained graph. Whole-run time divided by
# 10 000 is the per-cycle figure that goes in the arrays below. The other two
# targets time one source event per sample, called directly on the criterion
# thread — a different measurement, so read the slopes rather than the ratios.
#
# The values in place are a wingfoil engine reading — every series measured back to
# back on the machine described in `../images/lscpu-b.txt` (4-core 2.10 GHz
# Xeon VM). Point estimates, in nanoseconds.
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
import matplotlib.ticker as ticker

depths = list(range(1, 11))

# Per source event, called directly on the criterion thread. Neither baseline
# has a bench handshake to remove; see the README on what does and does not make
# these comparable to the wingfoil series.
async_s  = [152, 233, 364, 693, 1263, 2509, 5100, 9996, 19869, 38487]
reactive = [24, 65, 167, 292, 672, 1374, 2820, 5727, 11266, 22595]

# wingfoil, per cycle: whole-run time / 10 000 cycles, no harness underneath.
cyc_interp   = [87.0, 116.4, 135.5, 150.3, 179.7, 199.0, 217.5, 259.0, 257.4, 287.5]
cyc_compiled = [21.1, 22.2, 21.9, 23.7, 23.3, 24.5, 24.7, 23.5, 25.2, 23.9]
cyc_nested   = [73.8, 78.1, 80.1, 73.4, 78.6, 74.6, 81.8, 72.7, 80.5, 86.0]

INTERP_COLOR   = '#2196F3'
ISLAND_COLOR   = '#0D47A1'
COMPILED_COLOR = '#00897B'
ASYNC_COLOR    = '#FF9800'
RX_COLOR       = '#F44336'


def style(ax, ylabel, title, legend_size=11):
    ax.grid(True, which='major', linestyle='-', linewidth=0.8, alpha=0.6)
    ax.grid(True, which='minor', linestyle='--', linewidth=0.5, alpha=0.4)
    ax.set_axisbelow(True)
    ax.set_xticks(depths)
    ax.set_xlabel('Branch/recombine depth', fontsize=12)
    ax.set_ylabel(ylabel, fontsize=12)
    ax.set_title(title, fontsize=13, fontweight='bold')
    ax.legend(fontsize=legend_size)


def fmt_time(y, _):
    return f'{y:.0f} ns' if y < 1000 else f'{y/1000:.0f} µs'


def log_axis(ax):
    ax.set_yscale('log')
    ax.yaxis.set_major_locator(ticker.LogLocator(base=10))
    ax.yaxis.set_minor_locator(ticker.LogLocator(base=10, subs=[2, 3, 4, 5, 6, 7, 8, 9]))
    ax.yaxis.set_major_formatter(ticker.FuncFormatter(fmt_time))
    ax.yaxis.set_minor_formatter(ticker.NullFormatter())


def linear_axis(ax):
    ax.set_ylim(bottom=0)
    ax.yaxis.set_major_formatter(ticker.FuncFormatter(fmt_time))


def render(series, ylabel, title, stem, legend_size):
    """Draw the same series twice — linear for impact, log to read the low end.

    A linear axis is what makes the doubling visible as doubling: the per-path
    baselines go near-vertical while all three wingfoil lines flatten onto the
    floor. It is also unreadable below ~1 µs, which is where the crossovers and
    the whole separation between the wingfoil tiers live — hence both, with the
    README leading on the linear one and linking the log one for detail.
    """
    for suffix, axis in (('', linear_axis), ('_log', log_axis)):
        fig, ax = plt.subplots(figsize=(8, 5))
        for ys, marker, color, label in series:
            ax.plot(depths, ys, marker, color=color, linewidth=2,
                    markersize=5 if '--' in marker else 6, label=label)
        axis(ax)
        style(ax, ylabel, title, legend_size)
        fig.tight_layout()
        fig.savefig(f'{stem}{suffix}.png', dpi=150, bbox_inches='tight')
        plt.close(fig)


# --- Chart 1: the three wingfoil tiers, per cycle, linear scale -------------
#
# The engine on its own, on a linear axis: this is the O(N) claim itself — one
# more level is one more node, a fixed step up, not a doubling. (Compare the log
# axis in chart 2, where the per-path libraries need four decades.) Both
# compiled tiers are flat: their added node is straight-line code, so it costs
# about what the arithmetic costs rather than the interpreter's ~22 ns of
# dispatch.
fig2, ax2 = plt.subplots(figsize=(8, 5))

ax2.plot(depths, cyc_interp, 'o-', color=INTERP_COLOR, linewidth=2, markersize=6,
         label='wingfoil interpreted')
ax2.plot(depths, cyc_nested, 'D--', color=ISLAND_COLOR, linewidth=2, markersize=5,
         label='wingfoil compiled island (nested)')
ax2.plot(depths, cyc_compiled, 'v-', color=COMPILED_COLOR, linewidth=2, markersize=6,
         label='wingfoil compiled (whole program)')

ax2.set_ylim(bottom=0)
ax2.yaxis.set_major_formatter(ticker.FuncFormatter(fmt_time))

style(ax2, 'Cost per cycle (10 000-cycle run)',
      'Branch/recombine cost per cycle, all three engine tiers')
fig2.tight_layout()
fig2.savefig('per_cycle.png', dpi=150, bbox_inches='tight')

# --- Chart 2: the same tiers against the two per-path baselines -------------
#
# Mixed harnesses by construction: the wingfoil series are per cycle with
# nothing under them, the baselines are per source event called directly on the
# criterion thread. Neither side carries a bench handshake, which is as close to
# like-for-like as these three targets get, but a cycle and an event are still
# different units. Read the *slopes* — linear against doubling is the claim —
# and take the ratios with the caveat the README attaches to them.
render(
    [
        (cyc_interp, 'o-', INTERP_COLOR, 'wingfoil interpreted (per cycle)'),
        (cyc_nested, 'D--', ISLAND_COLOR, 'wingfoil compiled island (per cycle)'),
        (cyc_compiled, 'v-', COMPILED_COLOR, 'wingfoil compiled (per cycle)'),
        (async_s, 's-', ASYNC_COLOR, 'async streams (per event)'),
        (reactive, '^-', RX_COLOR, 'reactive / rxrust (per event)'),
    ],
    'Cost per cycle / event',
    'Topological sort vs per-path propagation: branch/recombine cost',
    'cross_library',
    9,
)

print("saved")
