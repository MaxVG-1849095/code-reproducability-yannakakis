select count(*) from yago0, yago2_1, yago2_2, yago2_3, yago2_4, yago6 where yago0.s = yago2_1.d and yago2_1.s = yago2_2.s and yago2_2.d = yago2_3.d and yago2_3.d = yago2_4.d and yago2_3.s = yago6.d;