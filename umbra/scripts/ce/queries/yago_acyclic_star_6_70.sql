select count(*) from yago0, yago11 yago11_1, yago11 yago11_2, yago11 yago11_3, yago2 where yago0.s = yago11_1.s and yago11_1.s = yago11_2.s and yago11_2.s = yago11_3.s and yago11_3.s = yago2.d;