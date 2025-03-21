baseline = [1.454633746,1.401958033,1.413946772,1.389880756,1.392712656,1.394949655,1.396576393,1.395505634,1.390080191,1.408487416]

baseline = [i * 1000 for i in baseline]

onethread = [1.502233066,1.484676426,1.491565097,1.436505157,1.428246431,1.439702116,1.463123918,1.448131844,1.438786359,1.448476443]

onethread = [i * 1000 for i in onethread]

twothreads = [1.432330403,1.318109037,1.378174317,1.268419716,1.473893088,1.309516803,1.334943637,1.653047605,1.453525396,1.512014714]

twothreads = [i * 1000 for i in twothreads]

fourthreads = [974.775247,947.137895,1052.941936,950.598021,924.639834,966.330591,944.966689,970.571349,935.320908,949.248055]

eightthreads = [1008.855651,1122.639552,943.619158,968.599059,939.976519,968.284837,968.576727,959.874705,952.666451,947.943384]

sixteenthreads = [1084.761105,983.339318,1086.788057,1029.79423,1063.326932,1121.556459,972.449832,1081.937029,963.804707,1052.649153]


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
ax.set_title('7b_norelease')
ax.legend(loc='upper right')
plt.show()