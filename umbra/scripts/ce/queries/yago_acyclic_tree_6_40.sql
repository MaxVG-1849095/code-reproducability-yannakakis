select count(*) from yago2 yago2_0, yago2 yago2_1, yago2 yago2_2, yago0, yago6, yago2 yago2_5 where yago2_0.s = yago2_1.s and yago2_1.s = yago2_2.s and yago2_2.s = yago6.d and yago2_1.d = yago0.s and yago0.s = yago2_5.d;