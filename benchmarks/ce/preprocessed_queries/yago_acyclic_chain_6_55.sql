select count(*) from yago2_0, yago6_1, yago6_2, yago2_3, yago2_4, yago2_5 where yago2_0.s = yago6_1.d and yago6_1.s = yago6_2.s and yago6_2.d = yago2_3.s and yago2_3.d = yago2_4.d and yago2_4.s = yago2_5.s;