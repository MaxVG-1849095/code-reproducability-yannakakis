select count(*) from yago2 yago2_0, yago2 yago2_1, yago0 yago0_2, yago0 yago0_3, yago2 yago2_4, yago2 yago2_5, yago2 yago2_6, yago2 yago2_7 where yago2_0.s = yago2_1.s and yago2_1.d = yago0_2.s and yago0_2.d = yago0_3.d and yago0_3.s = yago2_4.d and yago2_4.s = yago2_5.s and yago2_5.d = yago2_6.d and yago2_6.s = yago2_7.s;