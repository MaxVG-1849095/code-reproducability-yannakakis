select count(*) from yago8_0, yago3, yago8_2, yago2_3, yago2_4 where yago8_0.s = yago3.s and yago3.s = yago8_2.s and yago8_2.s = yago2_3.d and yago2_3.d = yago2_4.d;