select count(*) from yago2 yago2_0, yago2 yago2_1, yago23 yago23_2, yago36, yago0, yago3, yago54 yago54_6, yago54 yago54_7, yago12 yago12_8, yago17, yago23 yago23_10, yago12 yago12_11 where yago2_0.s = yago2_1.s and yago2_1.d = yago0.s and yago23_2.s = yago36.s and yago23_2.d = yago23_10.d and yago0.d = yago3.d and yago3.s = yago54_6.s and yago54_6.d = yago54_7.d and yago54_7.s = yago12_8.s and yago12_8.d = yago17.d and yago17.s = yago12_11.d and yago23_10.s = yago12_11.s;