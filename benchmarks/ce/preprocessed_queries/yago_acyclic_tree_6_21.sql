select count(*) from yago55, yago2_1, yago2_2, yago2_3, yago1, yago2_5 where yago55.s = yago2_1.d and yago2_1.s = yago2_2.s and yago2_2.s = yago2_3.s and yago2_2.d = yago1.s and yago2_3.d = yago2_5.d;