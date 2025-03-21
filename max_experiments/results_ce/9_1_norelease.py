baseline = [99.201746944,100.253119029,104.106857615,100.082294302,98.674669751]

onethread = [98.151578476,98.562141803,97.231201913,96.113745121,95.794442535]

twothreads = [92.280226636,51.645258072,91.703651096,95.908764244,99.941793634]

fourthreads = [32.967930959,32.049758383,46.777938745,31.730164141,31.812761429]

eightthreads = [30.810727322,30.886702587,32.127363053,32.218685375,33.907241628]

sixteenthreads = [35.290368035,34.845754887,32.965507558,31.405385973,31.617999716]


print(baseline)

# make plot
import matplotlib.pyplot as plt
fig, ax = plt.subplots()
ax.plot(baseline, label='baseline')
ax.plot(onethread, label='1 thread')
ax.plot(twothreads, label='2 threads')
ax.plot(fourthreads, label='4 threads')
ax.plot(eightthreads, label='8 threads')
ax.plot(sixteenthreads, label='16 threads')
ax.set_xlabel('Iteration')
ax.set_ylabel('Time (ms)')
ax.set_title('9_1_norelease')
ax.legend(loc='upper right')
plt.show()