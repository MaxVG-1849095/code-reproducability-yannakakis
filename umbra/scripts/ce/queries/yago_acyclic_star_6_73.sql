select count(*) from yago8 yago8_0, yago0, yago8 yago8_2, yago8 yago8_3, yago2 where yago8_0.s = yago0.s and yago0.s = yago8_2.s and yago8_2.s = yago8_3.s and yago8_3.s = yago2.d;