select count(*) from yago2 yago2_0, yago2 yago2_1, yago2 yago2_2, yago2 yago2_3, yago2 yago2_4 where yago2_0.s = yago2_1.s and yago2_0.d = yago2_2.d and yago2_2.s = yago2_3.s and yago2_3.s = yago2_4.s;