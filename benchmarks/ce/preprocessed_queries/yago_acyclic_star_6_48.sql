select count(*) from yago1, yago3, yago8_2, yago8_3, yago2_4, yago2_5 where yago1.s = yago3.s and yago3.s = yago8_2.s and yago8_2.s = yago8_3.s and yago8_3.s = yago2_4.d and yago2_4.d = yago2_5.d;