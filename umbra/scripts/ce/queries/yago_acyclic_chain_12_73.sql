select count(*) from yago2 yago2_0, yago2 yago2_1, yago63, yago11, yago2 yago2_4, yago2 yago2_5, yago0 yago0_6, yago52, yago0 yago0_8, yago0 yago0_9, yago2 yago2_10, yago2 yago2_11 where yago2_0.s = yago2_1.s and yago2_1.d = yago2_4.d and yago63.s = yago2_10.d and yago63.d = yago11.d and yago2_4.s = yago2_5.s and yago2_5.d = yago0_6.s and yago0_6.d = yago52.d and yago52.s = yago0_8.s and yago0_8.d = yago0_9.d and yago0_9.s = yago2_11.d and yago2_10.s = yago2_11.s;