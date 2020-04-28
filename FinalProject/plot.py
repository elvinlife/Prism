#!python3
# the difficulty is [0, 1, 0, ..., 0], [0, 8, 0, ..., 0], [0, 64, 0, ..., 0]
import matplotlib.pyplot as plt
topo_2_allblocks = {
    'line': [69, 491, 1452],
    'reg2': [99, 505, 920],
    'reg3': [123, 579, 679],
    'reg4': [122, 492, 624]
        }
topo_2_validblocks = {
    'line': [65, 361, 383],
    'reg2': [86, 183, 179],
    'reg3': [99, 206, 197],
    'reg4': [108, 197, 175]
        }

diff_2_allblocks = {
    '1': [69, 99, 123, 122],
    '8': [491, 505, 579, 492],
    '64': [1452, 920, 679, 624]
        }

diff_2_validblocks = {
        '1': [65, 86, 99, 108],
        '8': [361, 183, 206, 197],
        '64': [383, 179, 197, 175]
        }

import matplotlib
import matplotlib.pyplot as plt
import numpy as np

def autolabel(rects, ax):
    """Attach a text label above each bar in *rects*, displaying its height."""
    for rect in rects:
        height = rect.get_height()
        ax.annotate('{:.2f}'.format(float(height)), xy=(rect.get_x() + rect.get_width() / 2, height), xytext=(0, 3),  # 3 points vertical offset
                textcoords="offset points", ha='center', va='bottom')

def plot_topo():
    labels = ['1', '8', '64']
    x = np.arange(len(labels))  # the label locations
    width = 0.15  # the width of the bars
    fig, ax = plt.subplots()
    rects1 = ax.bar(x - 3/2 * width, np.array(topo_2_validblocks['line'])/60, width, label='Line')
    rects2 = ax.bar(x - width / 2, np.array(topo_2_validblocks['reg2'])/60, width, label='Reg-2')
    rects3 = ax.bar(x + width / 2, np.array(topo_2_validblocks['reg3'])/60, width, label='Reg-3')
    rects4 = ax.bar(x + 3/2 * width, np.array(topo_2_validblocks['reg4'])/60, width, label='Reg-4')
    # Add some text for labels, title and custom x-axis tick labels, etc.
    ax.set_ylabel('Mining Rate (tx/s)')
    ax.set_xlabel('Difficulty')
    ax.set_title('Mining rate ~ Topology')
    ax.set_xticks(x)
    ax.set_xticklabels(labels)
    ax.legend()

    autolabel(rects1, ax)
    autolabel(rects2, ax)
    autolabel(rects3, ax)
    autolabel(rects4, ax)
    fig.tight_layout()
    plt.savefig('topology.pdf')

def plot_diff():
    labels = ['Line', 'Reg-2', 'Reg-3', 'Reg-4']
    x = np.arange(len(labels))  # the label locations
    width = 0.15  # the width of the bars
    fig, ax = plt.subplots()
    rects1 = ax.bar(x - width, np.array(diff_2_validblocks['1'])/60, width, label='Difficulty: 1')
    rects2 = ax.bar(x, np.array(diff_2_validblocks['8'])/60, width, label='Difficulty: 8')
    rects3 = ax.bar(x + width, np.array(diff_2_validblocks['64'])/60, width, label='Difficulty: 64')
    ax.set_ylabel('Mining Rate (tx/s)')
    ax.set_xlabel('Topology')
    ax.set_title('Mining rate ~ Difficulty')
    ax.set_xticks(x)
    ax.set_xticklabels(labels)
    ax.legend()

    autolabel(rects1, ax)
    autolabel(rects2, ax)
    autolabel(rects3, ax)
    fig.tight_layout()
    plt.savefig('diffculty.pdf')

plot_diff()
plot_topo()
