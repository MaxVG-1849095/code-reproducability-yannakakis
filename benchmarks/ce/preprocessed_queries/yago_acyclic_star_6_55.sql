select count(*) from yago11, yago6_1, yago6_2, yago2_3, yago2_4, yago2_5 where yago11.s = yago6_1.s and yago6_1.s = yago6_2.s and yago6_2.s = yago2_3.d and yago2_3.d = yago2_4.d and yago2_4.d = yago2_5.d;