select count(*) from imdb1, imdb121, imdb2, imdb25 where imdb1.s = imdb121.s and imdb121.d = imdb2.d and imdb2.d = imdb25.s;