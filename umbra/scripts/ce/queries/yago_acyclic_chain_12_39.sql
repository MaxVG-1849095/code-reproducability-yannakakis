select count(*) from yago0, yago2 yago2_1, yago2 yago2_2, yago3, yago58, yago23 yago23_5, yago23 yago23_6, yago54, yago17, yago23 yago23_9, yago2 yago2_10, yago2 yago2_11 where yago0.d = yago3.d and yago2_1.s = yago2_2.s and yago2_1.d = yago2_10.d and yago3.s = yago58.s and yago58.d = yago23_5.s and yago23_5.d = yago23_6.d and yago23_6.s = yago54.s and yago54.d = yago17.d and yago17.s = yago23_9.s and yago23_9.d = yago2_11.d and yago2_10.s = yago2_11.s;