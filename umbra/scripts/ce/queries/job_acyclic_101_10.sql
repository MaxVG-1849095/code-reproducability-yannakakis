select count(*) from imdb2, imdb123, imdb1 where imdb2.d = imdb123.d and imdb123.s = imdb1.s;