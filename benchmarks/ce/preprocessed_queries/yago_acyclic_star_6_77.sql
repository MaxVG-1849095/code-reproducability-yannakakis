select count(*) from yago25_0, yago25_1, yago1, yago8, yago2 where yago25_0.s = yago25_1.s and yago25_1.s = yago1.s and yago1.s = yago8.s and yago8.s = yago2.d;